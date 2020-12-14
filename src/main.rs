// TEST BACKUP

//! Original author
//! Author: @termhn
//! Original repo: https://github.com/termhn/ggez_snake
//!
//! Edited by Joonas Lampinen 2020


// Next we need to actually `use` the pieces of ggez that we are going
// to need frequently.
use ggez;
use ggez::audio;
use ggez::audio::SoundSource;
use ggez::event;
use ggez::event::{KeyCode, KeyMods};
use ggez::graphics::Color;
use ggez::graphics::Scale;
use ggez::graphics::TextFragment;
use ggez::graphics::{self};
use ggez::nalgebra::Point2;
use ggez::{nalgebra as na, Context, GameResult};

// We'll bring in some things from `std` to help us in the future.
use std::collections::LinkedList;
use std::env;
use std::path;
use std::time::{Duration, Instant};

// And finally bring the `Rng` trait into scope so that we can generate
// some random numbers later.
use rand::Rng;

const DEBUG_ON: bool = true;

type Vector2 = na::Vector2<f32>;


// First we'll import the crates we need for our game;

#[derive(Copy, Clone)]
enum GameStates {
    GameOver,
    GameOn,
    Pause,
    Restart,
}

macro_rules! debug2 {
    (x => $e:expr) => {
        format!("{}={}, ", stringify!($e), $e)
    };
}

// Screen resolution / window size
pub struct Screen {
    pub size: Vector2,
}

impl Screen {
    fn size() -> Vector2 {
        return Vector2::new(1920.0, 1080.0);
    }
}

// Grid
struct Grid {
    // Background tiles
    spritebatch: graphics::spritebatch::SpriteBatch,

    // Wall tiles
    spritebatch2: graphics::spritebatch::SpriteBatch,
}

impl Grid {
    pub fn new(ctx: &mut Context) -> Self {
        // Background tiles
        let image = graphics::Image::new(ctx, "/png/element_grey_background.png").unwrap();
        let mut batch = graphics::spritebatch::SpriteBatch::new(image);

        // Add background tiles
        for x in 0..Grid::size().x as i16 {
            for y in 0..Grid::size().y as i16 {
                let x = x as f32;
                let y = y as f32;
                let p = graphics::DrawParam::new()
                    .dest(Point2::new(x * 32.0, y * 32.0))
                    .scale(Vector2::new(1.0, 1.0));
                batch.add(p);
            }
        }

        // Wall tiles
        let image2 = graphics::Image::new(ctx, "/png/element_grey_square.png").unwrap();
        let mut batch2 = graphics::spritebatch::SpriteBatch::new(image2);

        // Add walls to spritebatch

        // Add left and right walls
        for y in -1..Grid::size().y as i16 + 1 {
            let y = y as f32;

            // Add left wall
            let p = graphics::DrawParam::new()
                .dest(Point2::new(-1.0 * 32.0, y * 32.0))
                .scale(Vector2::new(1.0, 1.0));
            batch2.add(p);

            // Add right wall
            let p = graphics::DrawParam::new()
                .dest(Point2::new((Grid::size().x as i16) as f32 * 32.0, y * 32.0))
                .scale(Vector2::new(1.0, 1.0));
            batch2.add(p);
        }

        // Add top and bottom walls
        for x in 0..Grid::size().x as i16 {
            let x = x as f32;

            // Top wall
            let p = graphics::DrawParam::new()
                .dest(Point2::new(x * 32.0, -1.0 * 32.0))
                .scale(Vector2::new(1.0, 1.0));
            batch2.add(p);

            // Bottom wall
            let p = graphics::DrawParam::new()
                .dest(Point2::new(x * 32.0, (Grid::size().y as i16) as f32 * 32.0))
                .scale(Vector2::new(1.0, 1.0));
            batch2.add(p);
        }

        Grid {
            spritebatch: batch,
            spritebatch2: batch2,
        }
    }
    // The first thing we want to do is set up some variables that will help us out later.

    // Here we define the size of our game board in terms of how many grid
    // cells it will take up. We choose to make a 56 x 30 game board.
    fn size() -> Vector2 {
        Vector2::new(56.0, 30.0)
    }

    // Now we define the pixel size of each tile, which we make 32x32 pixels.
    const CELL_SIZE: i16 = 32;

    // Get pixel size of grid calculated from number of grid cells and cell size
    fn pixel_size() -> Vector2 {
        Grid::CELL_SIZE as f32 * Grid::size()
    }

    // Grid pixel offset
    fn offset() -> Vector2 {
        0.5 * (Screen::size() - Grid::pixel_size())
    }

    // Add images to spritebatch and draw grid.
    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        // Clear
        //self.spritebatch2.clear();

        // Transform and scale background
        let param = graphics::DrawParam::new()
            .dest(Point2::new(Grid::offset().x, Grid::offset().y))
            .scale(Vector2::new(1.0, 1.0));

        // Draw background
        graphics::draw(ctx, &self.spritebatch, param)?;

        // Clear background
        //self.spritebatch.clear();

        // Transform and scale walls
        let param = graphics::DrawParam::new()
            .dest(Point2::new(Grid::offset().x, Grid::offset().y))
            .scale(Vector2::new(1.0, 1.0));

        // Draw walls
        graphics::draw(ctx, &self.spritebatch2, param)?;

        Ok(())
    }
}

/// Now we define a struct that will hold an entity's position on our game board
/// or grid which we defined above. We'll use signed integers because we only want
/// to store whole numbers, and we need them to be signed so that they work properly
/// with our modulus arithmetic later.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct GridPosition {
    x: i16,
    y: i16,
}

/// This is a trait that provides a modulus function that works for negative values
/// rather than just the standard remainder op (%) which does not. We'll use this
/// to get our snake to wrap from one side of the game board around to the other
/// when it goes off the top, bottom, left, or right side of the screen.
trait ModuloSigned {
    fn modulo(&self, n: Self) -> Self;
}

/// Here we implement our `ModuloSigned` trait for any type T which implements
/// `Add` (the `+` operator) with an output type T and Rem (the `%` operator)
/// that also has an output type of T, and that can be cloned. These are the bounds
/// that we need in order to implement a modulus function that works for negative numbers
/// as well.
impl<T> ModuloSigned for T
where
    T: std::ops::Add<Output = T> + std::ops::Rem<Output = T> + Clone,
{
    fn modulo(&self, n: T) -> T {
        // Because of our trait bounds, we can now apply these operators.
        (self.clone() % n.clone() + n.clone()) % n.clone()
    }
}

impl GridPosition {
    /// We make a standard helper function so that we can create a new `GridPosition`
    /// more easily.
    pub fn new(x: i16, y: i16) -> Self {
        GridPosition { x, y }
    }

    /// As well as a helper function that will give us a random `GridPosition` from
    /// `(0, 0)` to `(max_x, max_y)`
    pub fn random(max_x: i16, max_y: i16) -> Self {
        let mut rng = rand::thread_rng();
        // We can use `.into()` to convert from `(i16, i16)` to a `GridPosition` since
        // we implement `From<(i16, i16)>` for `GridPosition` below.
        (
            rng.gen_range::<i16, i16, i16>(0, max_x),
            rng.gen_range::<i16, i16, i16>(0, max_y),
        )
            .into()
    }

    /// We'll make another helper function that takes one grid position and returns a new one after
    /// making one move in the direction of `dir`. We use our `SignedModulo` trait
    /// above, which is now implemented on `i16` because it satisfies the trait bounds,
    /// to automatically wrap around within our grid size if the move would have otherwise
    /// moved us off the board to the top, bottom, left, or right.
    pub fn new_from_move(pos: GridPosition, dir: Direction) -> Self {
        match dir {
            Direction::Up => GridPosition::new(pos.x, (pos.y - 1).modulo(Grid::size().y as i16)),
            Direction::Down => GridPosition::new(pos.x, (pos.y + 1).modulo(Grid::size().y as i16)),
            Direction::Left => GridPosition::new((pos.x - 1).modulo(Grid::size().x as i16), pos.y),
            Direction::Right => GridPosition::new((pos.x + 1).modulo(Grid::size().x as i16), pos.y),
        }
    }
}

/// We implement the `From` trait, which in this case allows us to convert easily between
/// a GridPosition and a ggez `graphics::Rect` which fills that grid cell.
/// Now we can just call `.into()` on a `GridPosition` where we want a
/// `Rect` that represents that grid cell.
impl From<GridPosition> for graphics::Rect {
    fn from(pos: GridPosition) -> Self {
        graphics::Rect::new_i32(
            (pos.x as i16 * Grid::CELL_SIZE).into(),
            (pos.y as i16 * Grid::CELL_SIZE).into(),
            Grid::CELL_SIZE.into(),
            Grid::CELL_SIZE.into(),
        )
    }
}

/// And here we implement `From` again to allow us to easily convert between
/// `(i16, i16)` and a `GridPosition`.
impl From<(i16, i16)> for GridPosition {
    fn from(pos: (i16, i16)) -> Self {
        GridPosition { x: pos.0, y: pos.1 }
    }
}

/// Next we create an enum that will represent all the possible
/// directions that our snake could move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    /// We create a helper function that will allow us to easily get the inverse
    /// of a `Direction` which we can use later to check if the player should be
    /// able to move the snake in a certain direction.
    pub fn inverse(&self) -> Self {
        match *self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }

    /// We also create a helper function that will let us convert between a
    /// `ggez` `Keycode` and the `Direction` that it represents. Of course,
    /// not every keycode represents a direction, so we return `None` if this
    /// is the case.
    pub fn from_keycode(key: KeyCode) -> Option<Direction> {
        match key {
            KeyCode::Up => Some(Direction::Up),
            KeyCode::Down => Some(Direction::Down),
            KeyCode::Left => Some(Direction::Left),
            KeyCode::Right => Some(Direction::Right),
            _ => None,
        }
    }
}

/// This is mostly just a semantic abstraction over a `GridPosition` to represent
/// a segment of the snake. It could be useful to, say, have each segment contain its
/// own color or something similar. This is an exercise left up to the reader ;)
#[derive(Clone, Copy, Debug)]
struct Segment {
    pos: GridPosition,
}

impl Segment {
    pub fn new(pos: GridPosition) -> Self {
        Segment { pos }
    }
}

/// This is again an abstraction over a `GridPosition` that represents
/// a piece of food the snake can eat. It can draw itself.
struct Food {
    pos: GridPosition,
    image: graphics::Image,
}

impl Food {
    pub fn new(ctx: &mut Context, pos: GridPosition) -> Self {
        let image = graphics::Image::new(ctx, "/png/element_red_square.png").unwrap();

        Food { pos, image }
    }

    /// Here is the first 1.0 we see what drawing looks like with ggez.
    /// We have a function that takes in a `&mut ggez::Context` which we use
    /// with the helpers in `ggez::graphics` to do drawing. We also return a
    /// `ggez::GameResult` so that we can use the `?` operator to bubble up
    /// failure of drawing.
    ///
    /// Note: this method of drawing does not scale. If you need to render
    /// a large number of shapes, use a SpriteBatch. This approach is fine for
    /// this example since there are a fairly limited number of calls.
    fn draw(&self, ctx: &mut Context) -> GameResult<()> {
        graphics::draw(
            ctx,
            &self.image,
            (ggez::mint::Vector2 {
                x: Grid::offset().x + Grid::CELL_SIZE as f32 * self.pos.x as f32,
                y: Grid::offset().y + Grid::CELL_SIZE as f32 * self.pos.y as f32,
            },),
        )
    }
}

/// Here we define an enum of the possible things that the snake could have "eaten"
/// during an update of the game. It could have either eaten a piece of `Food`, or
/// it could have eaten `Itself` if the head ran into its body.
#[derive(Clone, Copy, Debug)]
enum Ate {
    Itself,
    Food,
}

/// Now we make a struct that contains all the information needed to describe the
/// state of the Snake itself.
struct Snake {
    /// First we have the head of the snake, which is a single `Segment`.
    head: Segment,
    /// Then we have the current direction the snake is moving. This is
    /// the direction it will move when `update` is called on it.
    dir: Direction,
    /// Next we have the body, which we choose to represent as a `LinkedList`
    /// of `Segment`s.
    body: LinkedList<Segment>,
    /// Now we have a property that represents the result of the last update
    /// that was performed. The snake could have eaten nothing (None), Food (Some(Ate::Food)),
    /// or Itself (Some(Ate::Itself))
    ate: Option<Ate>,
    /// Finally we store the direction that the snake was traveling the last
    /// 1.0 that `update` was called, which we will use to determine valid
    /// directions that it could move the next 1.0 update is called.
    last_update_dir: Direction,
    /// Store the direction that will be used in the `update` after the next `update`
    /// This is needed so a user can press two directions (eg. left then up)
    /// before one `update` has happened. It sort of queues up key press input
    next_dir: Option<Direction>,

    points: i16,

    spritebatch: graphics::spritebatch::SpriteBatch,
}

impl Snake {
    pub fn new(ctx: &mut Context, pos: GridPosition) -> Self {
        let mut body = LinkedList::new();
        // Our snake will initially have a head and one body segment,
        // and will be moving to the right.
        body.push_back(Segment::new((pos.x - 1, pos.y).into()));

        let image = graphics::Image::new(ctx, "/png/element_green_square.png").unwrap();
        let batch = graphics::spritebatch::SpriteBatch::new(image);

        Snake {
            head: Segment::new(pos),
            dir: Direction::Right,
            last_update_dir: Direction::Right,
            body: body,
            ate: None,
            next_dir: None,
            points: 0,
            spritebatch: batch,
        }
    }

    /// A helper function that determines whether
    /// the snake eats a given piece of Food based
    /// on its current position
    fn eats(&self, food: &Food) -> bool {
        if self.head.pos == food.pos {
            true
        } else {
            false
        }
    }

    /// A helper function that determines whether
    /// the snake eats itself based on its current position
    fn eats_self(&self) -> bool {
        for seg in self.body.iter() {
            if self.head.pos == seg.pos {
                return true;
            }
        }
        false
    }

    /// The main update function for our snake which gets called every 1.0
    /// we want to update the game state.
    fn update(&mut self, food: &Food) {
        // If `last_update_dir` has already been updated to be the same as `dir`
        // and we have a `next_dir`, then set `dir` to `next_dir` and unset `next_dir`
        if self.last_update_dir == self.dir && self.next_dir.is_some() {
            self.dir = self.next_dir.unwrap();
            self.next_dir = None;
        }
        // First we get a new head position by using our `new_from_move` helper
        // function from earlier. We move our head in the direction we are currently
        // heading.
        let new_head_pos = GridPosition::new_from_move(self.head.pos, self.dir);
        // Next we create a new segment will be our new head segment using the
        // new position we just made.
        let new_head = Segment::new(new_head_pos);
        // Then we push our current head Segment onto the front of our body
        self.body.push_front(self.head);
        // And finally make our actual head the new Segment we created. This has
        // effectively moved the snake in the current direction.
        self.head = new_head;
        // Next we check whether the snake eats itself or some food, and if so,
        // we set our `ate` member to reflect that state.
        if self.eats_self() {
            self.ate = Some(Ate::Itself);
        } else if self.eats(food) {
            self.ate = Some(Ate::Food);
        } else {
            self.ate = None
        }
        // If we didn't eat anything this turn, we remove the last segment from our body,
        // which gives the illusion that the snake is moving. In reality, all the segments stay
        // stationary, we just add a segment to the front and remove one from the back. If we eat
        // a piece of food, then we leave the last segment so that we extend our body by one.
        if let None = self.ate {
            self.body.pop_back();
        }
        // And set our last_update_dir to the direction we just moved.
        self.last_update_dir = self.dir;
    }

    /// Here we have the Snake draw itself. This is very similar to how we saw the Food
    /// draw itself earlier.
    ///
    /// using SpriteBatch.
    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        // We first iterate through the body segments and draw them.
        for seg in self.body.iter() {
            let p = graphics::DrawParam::new()
                .dest(Point2::new(
                    (Grid::CELL_SIZE * seg.pos.x).into(),
                    (Grid::CELL_SIZE * seg.pos.y).into(),
                ))
                .scale(Vector2::new(1.0, 1.0));
            self.spritebatch.add(p);
        }
        let param = graphics::DrawParam::new()
            .dest(Point2::new(Grid::offset().x, Grid::offset().y))
            .scale(Vector2::new(1.0, 1.0));

        let p = graphics::DrawParam::new()
            .dest(Point2::new(
                (Grid::CELL_SIZE * self.head.pos.x).into(),
                (Grid::CELL_SIZE * self.head.pos.y).into(),
            ))
            .scale(Vector2::new(1.0, 1.0));
        self.spritebatch.add(p);

        graphics::draw(ctx, &self.spritebatch, param)?;
        self.spritebatch.clear();
        Ok(())
    }
}

/// Now we have the heart of our game, the GameState. This struct
/// will implement ggez's `EventHandler` trait and will therefore drive
/// everything else that happens in our game.
struct GameState {
    hit_sound: audio::Source,

    //music: audio::Source,
    grid: Grid,
    /// First we need a Snake
    snake: Snake,
    /// A piece of food
    food: Food,
    /// Whether the game is over or not
    _gameover: bool,
    /// And we track the last 1.0 we updated so that we can limit
    /// our update rate.
    last_update: Instant,

    text: graphics::Text,
    text_game_over: graphics::Text,
    text_try_again: graphics::Text,
    text_pause: graphics::Text,
    text_debug: graphics::Text,

    game_states: GameStates,
    music_on: bool,
    music: audio::Source,
}

impl GameState {
    /// Our new function will set up the initial state of our game.
    pub fn new(_ctx: &mut Context) -> GameResult<GameState> {
        // First we put our snake a quarter of the way across our grid in the x axis
        // and half way down the y axis. This works well since we start out moving to the right.
        let snake_pos = (Grid::size().x as i16 / 4, Grid::size().y as i16 / 2).into();
        // Then we choose a random place to put our piece of food using the helper we made
        // earlier.
        let food_pos = GridPosition::random(Grid::size().x as i16, Grid::size().y as i16);

        // The ttf file will be in your resources directory. Later, we
        // will mount that directory so we can omit it in the path here.
        let _font = graphics::Font::new(_ctx, "/DejaVuSerif.ttf");

        let mut hit_sound = audio::Source::new(_ctx, "/phaseJump5.mp3")?;

        hit_sound.set_volume(2.0);

        let mut music = audio::Source::new(_ctx, "/BoxCat_Games_-_10_-_Epic_Song.mp3")?;
        //let mut music = audio::Source::new(_ctx, "/phaseJump5.mp3")?;

        music.set_volume(0.2);
        music.set_repeat(true);

        //let _ = music.play_detached();
        let _ = music.play();

        let s = GameState {
            hit_sound,
            //music,
            grid: Grid::new(_ctx),
            snake: Snake::new(_ctx, snake_pos),
            food: Food::new(_ctx, food_pos),
            _gameover: false,
            last_update: Instant::now(),
            text: graphics::Text::new("Hello world!"),
            //text_game_over: graphics::Text::new("GAME OVER").scale(Scale::uniform(25.0)),
            text_game_over: graphics::Text::new(TextFragment {
                // `TextFragment` stores a string, and optional parameters which will override those
                // of `Text` itself. This allows inlining differently formatted lines, words,
                // or even individual letters, into the same block of text.
                text: "GAME OVER".to_string(),
                color: Some(Color::new(1.0, 0.0, 0.0, 1.0)),
                // `Font` is a handle to a loaded TTF, stored inside the `Context`.
                // `Font::default()` always exists and maps to DejaVuSerif.
                font: Some(graphics::Font::default()),
                scale: Some(Scale::uniform(100.0)),
                // This doesn't do anything at this point; can be used to omit fields in declarations.
                ..Default::default()
            }),
            text_try_again: graphics::Text::new(TextFragment {
                // `TextFragment` stores a string, and optional parameters which will override those
                // of `Text` itself. This allows inlining differently formatted lines, words,
                // or even individual letters, into the same block of text.
                text: "Do you want to try again? Y/N".to_string(),
                color: Some(Color::new(1.0, 1.0, 1.0, 1.0)),
                // `Font` is a handle to a loaded TTF, stored inside the `Context`.
                // `Font::default()` always exists and maps to DejaVuSerif.
                font: Some(graphics::Font::default()),
                scale: Some(Scale::uniform(30.0)),
                // This doesn't do anything at this point; can be used to omit fields in declarations.
                ..Default::default()
            }),
            text_pause: graphics::Text::new(TextFragment {
                // `TextFragment` stores a string, and optional parameters which will override those
                // of `Text` itself. This allows inlining differently formatted lines, words,
                // or even individual letters, into the same block of text.
                text: "PAUSED".to_string(),
                color: Some(Color::new(1.0, 1.0, 1.0, 1.0)),
                // `Font` is a handle to a loaded TTF, stored inside the `Context`.
                // `Font::default()` always exists and maps to DejaVuSerif.
                font: Some(graphics::Font::default()),
                scale: Some(Scale::uniform(100.0)),
                // This doesn't do anything at this point; can be used to omit fields in declarations.
                ..Default::default()
            }),
            text_debug: graphics::Text::new(TextFragment {
                // `TextFragment` stores a string, and optional parameters which will override those
                // of `Text` itself. This allows inlining differently formatted lines, words,
                // or even individual letters, into the same block of text.
                text: "DEBUG".to_string(),
                color: Some(Color::new(1.0, 1.0, 1.0, 1.0)),
                // `Font` is a handle to a loaded TTF, stored inside the `Context`.
                // `Font::default()` always exists and maps to DejaVuSerif.
                font: Some(graphics::Font::default()),
                scale: Some(Scale::uniform(14.0)),
                // This doesn't do anything at this point; can be used to omit fields in declarations.
                ..Default::default()
            }),
            game_states: GameStates::GameOn,
            music_on: true,
            music: music,
        };

        Ok(s)
    }
}

/// Now we implement EventHandler for GameState. This provides an interface
/// that ggez will call automatically when different events happen.
impl event::EventHandler for GameState {
    /// Update will happen on every frame before it is drawn. This is where we update
    /// our game state to react to whatever is happening in the game world.
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        // First we check to see if enough 1.0 has elapsed since our last update based on
        // the update rate we defined at the top.
        //if Instant::now() - self.last_update >= Duration::from_millis(MILLIS_PER_UPDATE) {
        if Instant::now() - self.last_update
            >= Duration::from_millis((100.0 - 8.0 * (self.snake.points as f32).sqrt()) as u64)
        {
            // Then we check to see if the game is over. If not, we'll update. If so, we'll just do nothing.
            self.text = graphics::Text::new(format!(
                "FPS: {:.0} Points: {}",
                ggez::timer::fps(_ctx),
                self.snake.points,
            ));

            /* let mut dbg = String::new();
            dbg.push_str(&debug2!(x => self.music_on));
            dbg.push_str(&debug2!(x => self.snake.points));
            dbg.push_str(&debug2!(x => Grid::size()));
            self.text_debug = graphics::Text::new(format!("{}", dbg)); */

            match self.game_states {
                GameStates::GameOver | GameStates::Pause => None,
                GameStates::Restart => Some({
                    self.snake.points = 0;
                    //self.snake.body.clear();
                    //self.snake.body.detach_all_nodes();
                    let snake_pos = (Grid::size().x as i16 / 4, Grid::size().y as i16 / 2).into();

                    self.snake = Snake::new(_ctx, snake_pos);

                    self.game_states = GameStates::GameOn;
                }),
                _ => Some({
                    // Here we do the actual updating of our game world. First we tell the snake to update itself,
                    // passing in a reference to our piece of food.
                    self.snake.update(&self.food);
                    // Next we check if the snake ate anything as it updated.
                    if let Some(ate) = self.snake.ate {
                        // If it did, we want to know what it ate.
                        match ate {
                            // If it ate a piece of food, we randomly select a new position for our piece of food
                            // and move it to this new position.
                            Ate::Food => {
                                let _ = self.hit_sound.play();
                                self.snake.points += 1;
                                let new_food_pos = GridPosition::random(
                                    Grid::size().x as i16,
                                    Grid::size().y as i16,
                                );
                                self.food.pos = new_food_pos;
                            }
                            // If it ate itself, we set our gameover state to true.
                            Ate::Itself => {
                                self.game_states = GameStates::GameOver;
                            }
                        }
                    }
                }),
            };

            // If we updated, we set our last_update to be now
            self.last_update = Instant::now();
        }

        //self.text = graphics::Text::new(format!("FPS: {}", ggez::timer::fps(_ctx)));

        // Finally we return `Ok` to indicate we didn't run into any errors
        Ok(())
    }

    /// draw is where we should actually render the game's current state.
    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        // First we clear the screen to a nice (well, maybe pretty glaring ;)) green
        graphics::clear(ctx, [0.2, 0.3, 0.6, 1.0].into());

        // Draw grid.
        self.grid.draw(ctx)?;

        // Then we tell the snake and the food to draw themselves
        self.snake.draw(ctx)?;
        self.food.draw(ctx)?;

        let dest_point = mint::Vector2 { x: (0.0), y: (0.0) };
        graphics::draw(ctx, &self.text, (dest_point,))?;

        match self.game_states {
            GameStates::GameOver => Some({
                let dest_point = mint::Vector2 {
                    x: 0.5 * Screen::size().x - 0.5 * self.text_game_over.width(ctx) as f32,
                    y: 0.5 * Screen::size().y - 0.5 * self.text_game_over.height(ctx) as f32,
                };
                graphics::draw(ctx, &self.text_game_over, (dest_point,))?;

                let dest_point = mint::Vector2 {
                    x: 0.5 * Screen::size().x - 0.5 * self.text_try_again.width(ctx) as f32,
                    y: 0.5 * Screen::size().y + 50.0,
                };
                graphics::draw(ctx, &self.text_try_again, (dest_point,))?;
            }),
            GameStates::Pause => Some({
                let dest_point = mint::Vector2 {
                    x: 0.5 * Screen::size().x - 0.5 * self.text_game_over.width(ctx) as f32,
                    y: 0.5 * Screen::size().y - 0.5 * self.text_game_over.height(ctx) as f32,
                };
                graphics::draw(ctx, &self.text_pause, (dest_point,))?;
            }),
            _ => None,
        };

        if DEBUG_ON {
            let dest_point = mint::Vector2 {
                x: 0.0,
                y: Screen::size().y - self.text_debug.height(ctx) as f32,
            };
            graphics::draw(ctx, &self.text_debug, (dest_point,))?;
        }

        // Finally we call graphics::present to 1.0 the gpu's framebuffer and display
        // the new frame we just drew.
        graphics::present(ctx)?;
        // We yield the current thread until the next update
        ggez::timer::yield_now();

        // And return success.
        Ok(())
    }

    /// key_down_event gets fired when a key gets pressed.
    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        keycode: KeyCode,
        _keymod: KeyMods,
        _repeat: bool,
    ) {
        // Here we attempt to convert the Keycode into a Direction using the helper
        // we defined earlier.
        if let Some(dir) = Direction::from_keycode(keycode) {
            // If it succeeds, we check if a new direction has already been set
            // and make sure the new direction is different then `snake.dir`
            if self.snake.dir != self.snake.last_update_dir && dir.inverse() != self.snake.dir {
                self.snake.next_dir = Some(dir);
            } else if dir.inverse() != self.snake.last_update_dir {
                // If no new direction has been set and the direction is not the inverse
                // of the `last_update_dir`, then set the snake's new direction to be the
                // direction the user pressed.
                self.snake.dir = dir;
            }
        }

        _ctx.continuing = match keycode {
            KeyCode::Q | KeyCode::Escape => false,
            _ => true,
        };

        match keycode {
            KeyCode::P => Some(
                self.game_states = match self.game_states {
                    GameStates::Pause => GameStates::GameOn,
                    GameStates::GameOn => GameStates::Pause,
                    _ => self.game_states,
                },
            ),
            KeyCode::M => Some({
                self.music_on = !self.music_on;

                match self.music_on {
                    true => Some({
                        self.music.resume();
                        //self.music.play_detached();
                        //self.music.
                    }),
                    false => Some({
                        self.music.pause();
                    }),
                };
            }),
            _ => None,
        };

        match self.game_states {
            GameStates::GameOver => Some({
                match keycode {
                    KeyCode::N => Some(_ctx.continuing = false),
                    KeyCode::Y => Some(self.game_states = GameStates::Restart),
                    _ => None,
                };
            }),
            _ => None,
        };
    }
}

fn main() -> GameResult {
    let resource_dir = if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let mut path = path::PathBuf::from(manifest_dir);
        path.push("resources");
        path
    } else {
        path::PathBuf::from("./resources")
    };

    // Here we use a ContextBuilder to setup metadata about our game. First the title and author
    let (ctx, events_loop) = &mut ggez::ContextBuilder::new("snake_remix", "Joonas Lampinen")
        // Next we set up the window. This title will be displayed in the title bar of the window.
        .window_setup(ggez::conf::WindowSetup::default().title("Snake Remix!"))
        // Now we get to set the size of the window, which we use our SCREEN_SIZE constant from earlier to help with
        .window_mode(
            ggez::conf::WindowMode::default().dimensions(Screen::size().x, Screen::size().y),
        )
        // And finally we attempt to build the context and create the window. If it fails, we panic with the message
        // "Failed to build ggez context"
        .add_resource_path(resource_dir)
        .build()?;

    let window = graphics::window(ctx);
    let monitor = window.get_current_monitor();
    window.set_fullscreen(Some(monitor));

    // Next we create a new instance of our GameState struct, which implements EventHandler
    let state = &mut GameState::new(ctx)?;

    // And finally we actually run our game, passing in our context and state.
    event::run(ctx, events_loop, state)
}
