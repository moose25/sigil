use clap::{Parser, Subcommand};

use sigil::color::{ColorMode, Rgb};
use sigil::fonts;
use sigil::gradient::{Direction, Gradient};
use sigil::render::{paint, Align, Banner, RenderOptions};

/// Give your CLI a face — modern gradient ASCII banners.
#[derive(Parser)]
#[command(name = "sigil", version, about, long_about = None)]
struct Cli {
    /// Text to render as a banner.
    #[arg(value_name = "TEXT")]
    text: Option<String>,

    /// Named gradient preset (see `sigil gradients`).
    #[arg(short, long, default_value = "ocean")]
    gradient: String,

    /// Custom gradient as comma-separated hex stops, e.g. "#ff5f6d,#ffc371".
    /// Overrides --gradient.
    #[arg(short = 'c', long)]
    colors: Option<String>,

    /// Sweep direction: horizontal | vertical | diagonal.
    #[arg(short, long, default_value = "horizontal")]
    direction: String,

    /// Alignment within the terminal width: left | center | right.
    #[arg(short, long, default_value = "left")]
    align: String,

    /// Font name (see `sigil fonts`).
    #[arg(short, long, default_value = "standard")]
    font: String,

    /// Target width for alignment (defaults to terminal width).
    #[arg(short = 'w', long)]
    width: Option<usize>,

    /// Blank lines above and below the banner.
    #[arg(short = 'm', long, default_value_t = 0)]
    margin: usize,

    /// Disable color output.
    #[arg(long)]
    no_color: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// List and preview built-in gradient presets.
    Gradients,
    /// List available fonts.
    Fonts,
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("sigil: {e}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), String> {
    let mode = if cli.no_color {
        ColorMode::None
    } else {
        ColorMode::detect()
    };

    match cli.command {
        Some(Command::Gradients) => list_gradients(mode),
        Some(Command::Fonts) => list_fonts(mode),
        None => {
            let text = cli
                .text
                .as_deref()
                .ok_or("no text given. Try: sigil \"My Project\"  (or `sigil --help`)")?;
            render_banner(&cli, text, mode)
        }
    }
}

fn render_banner(cli: &Cli, text: &str, mode: ColorMode) -> Result<(), String> {
    let font = fonts::load(&cli.font)?;
    let gradient = resolve_gradient(cli)?;
    let direction = Direction::parse(&cli.direction)?;
    let align = Align::parse(&cli.align)?;
    let target_width = cli.width.unwrap_or_else(term_width);

    let banner = Banner::layout(&font, text)?;
    let opts = RenderOptions {
        gradient,
        direction,
        align,
        mode,
        target_width,
        margin_y: cli.margin,
    };
    print!("{}", paint(&banner, &opts));
    Ok(())
}

/// Build the gradient from --colors (if given) or the named --gradient preset.
fn resolve_gradient(cli: &Cli) -> Result<Gradient, String> {
    if let Some(list) = &cli.colors {
        let stops = list
            .split(',')
            .map(|s| Rgb::parse(s.trim()))
            .collect::<Result<Vec<_>, _>>()?;
        if stops.is_empty() {
            return Err("--colors needs at least one hex stop".into());
        }
        return Ok(Gradient::new(&stops));
    }
    Gradient::preset(&cli.gradient)
        .ok_or_else(|| format!("unknown gradient: {}. See `sigil gradients`.", cli.gradient))
}

fn list_gradients(mode: ColorMode) -> Result<(), String> {
    println!("Built-in gradients:\n");
    for name in Gradient::preset_names() {
        let g = Gradient::preset(name).unwrap();
        let bar = swatch(&g, mode, 24);
        println!("  {bar}  {name}");
    }
    println!("\nCustom: --colors \"#ff5f6d,#ffc371\"");
    Ok(())
}

/// Render a horizontal preview bar for a gradient.
fn swatch(g: &Gradient, mode: ColorMode, width: usize) -> String {
    let mut s = String::new();
    for i in 0..width {
        let t = if width <= 1 {
            0.0
        } else {
            i as f32 / (width - 1) as f32
        };
        let c = g.sample(t);
        if mode == ColorMode::None {
            s.push('#');
        } else {
            s.push_str(&mode.fg(c));
            s.push('\u{2588}'); // full block
        }
    }
    s.push_str(mode.reset());
    s
}

fn list_fonts(mode: ColorMode) -> Result<(), String> {
    let gradient = Gradient::preset("ocean").unwrap();
    for info in fonts::catalog() {
        println!("\n\x1b[1m{}\x1b[0m — {}", info.name, info.description);
        let font = fonts::load(info.name)?;
        let banner = Banner::layout(&font, "Sigil")?;
        let opts = RenderOptions {
            gradient: gradient.clone(),
            direction: Direction::Horizontal,
            align: Align::Left,
            mode,
            target_width: 0,
            margin_y: 0,
        };
        print!("{}", paint(&banner, &opts));
    }
    Ok(())
}

/// Best-effort terminal width; falls back to 80 columns.
fn term_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&w| w > 0)
        .unwrap_or(80)
}
