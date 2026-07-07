use std::io::{IsTerminal, Read};
use std::path::Path;

use clap::{Parser, Subcommand};

use sigil::animate::{self, Anim};
use sigil::color::{ColorMode, Rgb};
use sigil::config::Config;
use sigil::export::{self, Format};
use sigil::fonts;
use sigil::gradient::{Direction, Gradient};
use sigil::render::{paint, Align, Banner, Border, RenderOptions};

/// Give your CLI a face — modern gradient ASCII banners.
#[derive(Parser)]
#[command(name = "sigil", version, about, long_about = None)]
struct Cli {
    /// Text to render as a banner (multiple words are joined with spaces).
    #[arg(value_name = "TEXT")]
    text: Vec<String>,

    /// Named gradient preset (see `sigil gradients`). [default: ocean]
    #[arg(short, long)]
    gradient: Option<String>,

    /// Custom gradient as comma-separated hex stops, e.g. "#ff5f6d,#ffc371".
    /// Overrides --gradient.
    #[arg(short = 'c', long)]
    colors: Option<String>,

    /// Sweep direction: horizontal | vertical | diagonal. [default: horizontal]
    #[arg(short, long)]
    direction: Option<String>,

    /// Sweep angle in degrees (0 = left→right, 90 = top→bottom). Overrides --direction.
    #[arg(long, allow_hyphen_values = true)]
    angle: Option<f32>,

    /// Reverse the gradient direction.
    #[arg(long)]
    reverse: bool,

    /// Repeat the gradient palette N times across the banner. [default: 1]
    #[arg(long)]
    cycle: Option<u32>,

    /// Frame the banner: none | round | single | double | heavy | ascii. [default: none]
    #[arg(short = 'b', long)]
    border: Option<String>,

    /// Interior padding between the banner and its frame (default 1 with a border).
    #[arg(short = 'p', long)]
    padding: Option<usize>,

    /// Solid frame color as a hex value (default: share the gradient).
    #[arg(long, value_name = "HEX")]
    border_color: Option<String>,

    /// Alignment within the terminal width: left | center | right. [default: left]
    #[arg(short, long)]
    align: Option<String>,

    /// Font name (see `sigil fonts`). [default: standard]
    #[arg(short, long)]
    font: Option<String>,

    /// Target width for alignment (defaults to terminal width).
    #[arg(short = 'w', long)]
    width: Option<usize>,

    /// Blank lines above and below the banner. [default: 0]
    #[arg(short = 'm', long)]
    margin: Option<usize>,

    /// Disable color output.
    #[arg(long)]
    no_color: bool,

    /// Output format: term | ansi | raw | rust | go | python | shell. [default: term]
    #[arg(short = 'F', long)]
    format: Option<String>,

    /// Write output to a file instead of stdout.
    #[arg(short = 'o', long, value_name = "FILE")]
    out: Option<std::path::PathBuf>,

    /// Animate the reveal on a terminal: none | sweep | type. [default: none]
    #[arg(long)]
    animate: Option<String>,

    /// Animation speed in frames per second (1-120). [default: 30]
    #[arg(long)]
    fps: Option<u32>,

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
    match cli.command {
        Some(Command::Gradients) => list_gradients(base_mode(cli.no_color)),
        Some(Command::Fonts) => list_fonts(base_mode(cli.no_color)),
        None => {
            let text = resolve_text(&cli.text)?;
            let config = Config::load()?;
            let settings = Settings::resolve(&cli, config);
            render_banner(&settings, &text)
        }
    }
}

/// Color mode for the list/preview subcommands: like the main render, color
/// only when writing to a real terminal (and not disabled).
fn base_mode(no_color: bool) -> ColorMode {
    if no_color || !std::io::stdout().is_terminal() {
        ColorMode::None
    } else {
        ColorMode::detect()
    }
}

/// Effective options after merging CLI flags over config files over built-in
/// defaults (flag > project config > user config > default).
struct Settings {
    gradient: String,
    colors: Option<String>,
    font: String,
    direction: String,
    align: String,
    angle: Option<f32>,
    reverse: bool,
    cycle: u32,
    border: String,
    padding: Option<usize>,
    border_color: Option<String>,
    margin: usize,
    width: Option<usize>,
    format: String,
    out: Option<std::path::PathBuf>,
    animate: String,
    fps: u32,
    no_color: bool,
}

impl Settings {
    fn resolve(cli: &Cli, cfg: Config) -> Settings {
        // A CLI flag wins; otherwise the config value; otherwise `default`.
        let pick = |flag: &Option<String>, from_cfg: Option<String>, default: &str| {
            flag.clone()
                .or(from_cfg)
                .unwrap_or_else(|| default.to_string())
        };
        Settings {
            gradient: pick(&cli.gradient, cfg.gradient, "ocean"),
            colors: cli.colors.clone().or(cfg.colors),
            font: pick(&cli.font, cfg.font, "standard"),
            direction: pick(&cli.direction, cfg.direction, "horizontal"),
            align: pick(&cli.align, cfg.align, "left"),
            angle: cli.angle.or(cfg.angle),
            reverse: cli.reverse || cfg.reverse.unwrap_or(false),
            cycle: cli.cycle.or(cfg.cycle).unwrap_or(1),
            border: pick(&cli.border, cfg.border, "none"),
            padding: cli.padding.or(cfg.padding),
            border_color: cli.border_color.clone().or(cfg.border_color),
            margin: cli.margin.or(cfg.margin).unwrap_or(0),
            width: cli.width.or(cfg.width),
            format: pick(&cli.format, cfg.format, "term"),
            out: cli.out.clone(),
            animate: pick(&cli.animate, cfg.animate, "none"),
            fps: cli.fps.or(cfg.fps).unwrap_or(30),
            no_color: cli.no_color,
        }
    }
}

/// Determine the banner text: the positional arguments joined with spaces, or
/// stdin when no arguments are given and it is piped/redirected. Whitespace
/// (including newlines) is collapsed to single spaces so piped input renders as
/// one line.
fn resolve_text(args: &[String]) -> Result<String, String> {
    if !args.is_empty() {
        return Ok(args.join(" "));
    }
    if std::io::stdin().is_terminal() {
        return Err(
            "no text given. Try: sigil \"My Project\"  or pipe it: echo hi | sigil".to_string(),
        );
    }
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| format!("failed to read stdin: {e}"))?;
    let text = buf.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        return Err("no text on stdin".to_string());
    }
    Ok(text)
}

fn render_banner(s: &Settings, text: &str) -> Result<(), String> {
    let format = Format::parse(&s.format)?;
    let font = fonts::load(&s.font)?;
    let gradient = resolve_gradient(s)?;
    let direction = match s.angle {
        Some(deg) => Direction::Angle(deg),
        None => Direction::parse(&s.direction)?,
    };
    let align = Align::parse(&s.align)?;
    let banner = Banner::layout(&font, text)?;
    let mode = color_mode(s, format);
    let anim = Anim::parse(&s.animate)?;

    // Animate only for live terminal output; snippets/files/pipes render static.
    if anim.is_animated()
        && format == Format::Term
        && s.out.is_none()
        && std::io::stdout().is_terminal()
    {
        let mut out = std::io::stdout().lock();
        return animate::play(&mut out, &banner, &gradient, mode, anim, s.fps)
            .map_err(|e| format!("animation error: {e}"));
    }

    let border = Border::parse(&s.border)?;
    // Give a framed banner a little breathing room by default.
    let padding = if border.is_some() {
        let p = s.padding.unwrap_or(1);
        (p + 1, p)
    } else {
        let p = s.padding.unwrap_or(0);
        (p, p)
    };
    let border_color = match &s.border_color {
        Some(hex) => Some(Rgb::parse(hex)?),
        None => None,
    };

    // Framed width includes the border and padding.
    let framed_w = banner.width + 2 * padding.0 + if border.is_some() { 2 } else { 0 };
    // Only direct terminal output gets terminal-width indentation and margins;
    // snippets and raw/ansi output stay tight to the banner's own width.
    let (target_width, margin_y) = if format == Format::Term {
        (s.width.unwrap_or_else(term_width), s.margin)
    } else {
        (framed_w, 0)
    };

    let opts = RenderOptions {
        gradient,
        direction,
        align,
        mode,
        target_width,
        margin_y,
        reverse: s.reverse,
        cycle: s.cycle,
        border,
        padding,
        border_color,
    };
    let painted = paint(&banner, &opts);
    let output = export::wrap(format, &painted);
    write_output(s.out.as_deref(), &output)
}

/// Decide the color mode for a render, given the format and where output goes.
///
/// `--no-color`/`NO_COLOR` always win. Snippet and `ansi` formats bake color
/// in. Plain terminal output uses color only when writing to an actual TTY.
fn color_mode(s: &Settings, format: Format) -> ColorMode {
    if s.no_color || std::env::var_os("NO_COLOR").is_some() {
        return ColorMode::None;
    }
    if format == Format::Raw {
        return ColorMode::None;
    }
    if format.forces_color() {
        return ColorMode::supported();
    }
    // Format::Term: color only on a real terminal (not piped, not a file).
    if s.out.is_none() && std::io::stdout().is_terminal() {
        ColorMode::supported()
    } else {
        ColorMode::None
    }
}

/// Write to a file when `-o` is given, otherwise to stdout.
fn write_output(path: Option<&Path>, content: &str) -> Result<(), String> {
    match path {
        Some(p) => {
            std::fs::write(p, content).map_err(|e| format!("cannot write {}: {e}", p.display()))?;
            eprintln!("wrote {}", p.display());
            Ok(())
        }
        None => {
            print!("{content}");
            Ok(())
        }
    }
}

/// Build the gradient from --colors (if given) or the named --gradient preset.
fn resolve_gradient(s: &Settings) -> Result<Gradient, String> {
    if let Some(list) = &s.colors {
        let stops = list
            .split(',')
            .map(|c| Rgb::parse(c.trim()))
            .collect::<Result<Vec<_>, _>>()?;
        if stops.is_empty() {
            return Err("--colors needs at least one hex stop".into());
        }
        return Ok(Gradient::new(&stops));
    }
    Gradient::preset(&s.gradient)
        .ok_or_else(|| format!("unknown gradient: {}. See `sigil gradients`.", s.gradient))
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
        preview_font(info.name, info.description, &gradient, mode)?;
    }
    let user = fonts::user_font_names();
    if !user.is_empty() {
        println!("\n{}", bold("User fonts (~/.config/sigil/fonts):", mode));
        for name in user {
            preview_font(&name, "custom", &gradient, mode)?;
        }
    }
    Ok(())
}

/// Wrap text in bold, unless color is disabled.
fn bold(text: &str, mode: ColorMode) -> String {
    if mode == ColorMode::None {
        text.to_string()
    } else {
        format!("\x1b[1m{text}\x1b[0m")
    }
}

fn preview_font(
    name: &str,
    description: &str,
    gradient: &Gradient,
    mode: ColorMode,
) -> Result<(), String> {
    println!("\n{} — {description}", bold(name, mode));
    let font = fonts::load(name)?;
    let banner = Banner::layout(&font, "Sigil")?;
    let opts = RenderOptions {
        gradient: gradient.clone(),
        direction: Direction::Horizontal,
        align: Align::Left,
        mode,
        target_width: 0,
        margin_y: 0,
        reverse: false,
        cycle: 1,
        border: None,
        padding: (0, 0),
        border_color: None,
    };
    print!("{}", paint(&banner, &opts));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_multi_word_text() {
        let args = vec!["Hello".to_string(), "World".to_string()];
        assert_eq!(resolve_text(&args).unwrap(), "Hello World");
    }
}
