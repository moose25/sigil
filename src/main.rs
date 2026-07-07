use std::io::{IsTerminal, Read, Write};
use std::path::Path;

use clap::{CommandFactory, Parser, Subcommand};

use sigil::animate::{self, Anim};
use sigil::color::{ColorMode, Rgb};
use sigil::config::Config;
use sigil::export::{self, Format};
use sigil::fonts;
use sigil::gradient::{Direction, Gradient, Interp};
use sigil::render::{paint, Align, Banner, Border, ColorBy, RenderOptions};
use sigil::themes::{self, Theme};

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

    /// Solid background fill behind the banner, as a hex value.
    #[arg(long, visible_alias = "bg", value_name = "HEX")]
    background: Option<String>,

    /// Caption to embed in the top border (needs a border).
    #[arg(long, value_name = "TEXT")]
    title: Option<String>,

    /// Draw a drop shadow behind the glyphs.
    #[arg(long)]
    shadow: bool,

    /// Drop-shadow color as a hex value (default: dark gray).
    #[arg(long, value_name = "HEX")]
    shadow_color: Option<String>,

    /// Draw an outline (halo) around the glyphs.
    #[arg(long)]
    outline: bool,

    /// Outline color as a hex value (default: near-black).
    #[arg(long, value_name = "HEX")]
    outline_color: Option<String>,

    /// Alignment within the terminal width: left | center | right. [default: left]
    #[arg(short, long)]
    align: Option<String>,

    /// How to map the gradient: banner | line | char. [default: banner]
    #[arg(long)]
    color_by: Option<String>,

    /// Gradient blend space: oklab | rgb | hsl. [default: oklab]
    #[arg(long)]
    interpolate: Option<String>,

    /// Font name (see `sigil fonts`). [default: standard]
    #[arg(short, long)]
    font: Option<String>,

    /// Colorize an existing ASCII-art file instead of rendering a font
    /// ("-" reads stdin). Overrides TEXT/--font.
    #[arg(long, value_name = "FILE")]
    art: Option<std::path::PathBuf>,

    /// Apply a named theme bundle (see `sigil themes`).
    #[arg(short = 't', long)]
    theme: Option<String>,

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

    /// PNG scale factor (1-10), for higher-resolution -F png output. [default: 1]
    #[arg(long)]
    scale: Option<usize>,

    /// Animate the reveal on a terminal: none | sweep | type. [default: none]
    #[arg(long)]
    animate: Option<String>,

    /// Animation speed in frames per second (1-120). [default: 30]
    #[arg(long)]
    fps: Option<u32>,

    /// Render each input line (or argument) as its own stacked banner.
    #[arg(short = 'l', long)]
    lines: bool,

    /// Also copy the output to the system clipboard.
    #[arg(long)]
    copy: bool,

    /// Pick a random font and gradient for any not explicitly set.
    #[arg(long)]
    random: bool,

    /// Seed for --random, for reproducible output.
    #[arg(long)]
    seed: Option<u64>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// List and preview built-in gradient presets.
    Gradients,
    /// List available fonts.
    Fonts,
    /// List available themes.
    Themes,
    /// Write a starter config file (or print it with --print).
    Init {
        /// Overwrite an existing config file.
        #[arg(long)]
        force: bool,
        /// Print the starter config to stdout instead of writing a file.
        #[arg(long)]
        print: bool,
    },
    /// Print a shell completion script (bash|zsh|fish|powershell|elvish).
    Completions {
        #[arg(value_name = "SHELL")]
        shell: clap_complete::Shell,
    },
    /// Print a man page (roff) to stdout.
    Man,
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
        Some(Command::Gradients) => list_gradients(base_mode(cli.no_color), &Config::load()?),
        Some(Command::Fonts) => list_fonts(base_mode(cli.no_color)),
        Some(Command::Themes) => {
            list_themes(&Config::load()?);
            Ok(())
        }
        Some(Command::Init { force, print }) => init_config(force, print),
        Some(Command::Completions { shell }) => {
            print_completions(shell);
            Ok(())
        }
        Some(Command::Man) => print_man(),
        None => {
            // With --art the content comes from a file/stdin, not the positional text.
            let text = if cli.art.is_some() {
                String::new()
            } else {
                resolve_text(&cli.text, cli.lines)?
            };
            let config = Config::load()?;
            let settings = Settings::resolve(&cli, config)?;
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
    background: Option<String>,
    margin: usize,
    width: Option<usize>,
    format: String,
    out: Option<std::path::PathBuf>,
    animate: String,
    fps: u32,
    no_color: bool,
    lines: bool,
    copy: bool,
    art: Option<std::path::PathBuf>,
    color_by: String,
    interpolate: String,
    title: Option<String>,
    shadow: bool,
    shadow_color: Option<String>,
    outline: bool,
    outline_color: Option<String>,
    scale: usize,
    user_gradients: std::collections::HashMap<String, Vec<String>>,
}

impl Settings {
    fn resolve(cli: &Cli, cfg: Config) -> Result<Settings, String> {
        // Resolve the theme (user config overrides a built-in of the same name).
        let theme = match &cli.theme {
            Some(name) => cfg
                .themes
                .get(name)
                .cloned()
                .or_else(|| themes::builtin(name))
                .ok_or_else(|| format!("unknown theme: {name}. See `sigil themes`."))?,
            None => Theme::default(),
        };

        // Precedence for each option: flag > theme > config > default.
        let pick = |flag: &Option<String>,
                    theme_v: Option<String>,
                    cfg_v: Option<String>,
                    default: &str| {
            flag.clone()
                .or(theme_v)
                .or(cfg_v)
                .unwrap_or_else(|| default.to_string())
        };
        // --random fills any still-unset font/gradient (over theme/config, not flags).
        let mut rng = cli.random.then(|| SplitMix::new(random_seed(cli.seed)));
        let rand_gradient = rng.as_mut().map(|r| choose(Gradient::preset_names(), r));
        let rand_font = rng.as_mut().map(|r| choose(&font_names(), r));

        Ok(Settings {
            gradient: cli
                .gradient
                .clone()
                .or(rand_gradient)
                .or(theme.gradient)
                .or(cfg.gradient)
                .unwrap_or_else(|| "ocean".to_string()),
            colors: cli.colors.clone().or(theme.colors).or(cfg.colors),
            font: cli
                .font
                .clone()
                .or(rand_font)
                .or(theme.font)
                .or(cfg.font)
                .unwrap_or_else(|| "standard".to_string()),
            direction: pick(&cli.direction, theme.direction, cfg.direction, "horizontal"),
            align: pick(&cli.align, theme.align, cfg.align, "left"),
            angle: cli.angle.or(theme.angle).or(cfg.angle),
            reverse: cli.reverse || theme.reverse.unwrap_or(false) || cfg.reverse.unwrap_or(false),
            cycle: cli.cycle.or(theme.cycle).or(cfg.cycle).unwrap_or(1),
            border: pick(&cli.border, theme.border, cfg.border, "none"),
            padding: cli.padding.or(theme.padding).or(cfg.padding),
            border_color: cli
                .border_color
                .clone()
                .or(theme.border_color)
                .or(cfg.border_color),
            background: cli
                .background
                .clone()
                .or(theme.background)
                .or(cfg.background),
            margin: cli.margin.or(cfg.margin).unwrap_or(0),
            width: cli.width.or(cfg.width),
            format: pick(&cli.format, None, cfg.format, "term"),
            out: cli.out.clone(),
            animate: pick(&cli.animate, None, cfg.animate, "none"),
            fps: cli.fps.or(cfg.fps).unwrap_or(30),
            no_color: cli.no_color,
            lines: cli.lines,
            copy: cli.copy,
            art: cli.art.clone(),
            color_by: pick(&cli.color_by, None, cfg.color_by, "banner"),
            interpolate: pick(&cli.interpolate, None, cfg.interpolate, "oklab"),
            title: cli.title.clone().or(cfg.title),
            shadow: cli.shadow || theme.shadow.unwrap_or(false) || cfg.shadow.unwrap_or(false),
            shadow_color: cli
                .shadow_color
                .clone()
                .or(theme.shadow_color)
                .or(cfg.shadow_color),
            outline: cli.outline || theme.outline.unwrap_or(false) || cfg.outline.unwrap_or(false),
            outline_color: cli
                .outline_color
                .clone()
                .or(theme.outline_color)
                .or(cfg.outline_color),
            scale: cli.scale.or(cfg.scale).unwrap_or(1),
            user_gradients: cfg.gradients,
        })
    }
}

/// Determine the banner text: the positional arguments joined with spaces, or
/// stdin when no arguments are given and it is piped/redirected. Whitespace
/// (including newlines) is collapsed to single spaces so piped input renders as
/// one line.
fn resolve_text(args: &[String], lines: bool) -> Result<String, String> {
    if !args.is_empty() {
        // In --lines mode each argument is its own line; otherwise join with spaces.
        return Ok(args.join(if lines { "\n" } else { " " }));
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
    let text = if lines {
        // Preserve line breaks; collapse spaces within each line.
        buf.lines()
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        buf.split_whitespace().collect::<Vec<_>>().join(" ")
    };
    if text.trim().is_empty() {
        return Err("no text on stdin".to_string());
    }
    Ok(text)
}

fn render_banner(s: &Settings, text: &str) -> Result<(), String> {
    let format = Format::parse(&s.format)?;
    let font = fonts::load(&s.font)?;
    let gradient = resolve_gradient(s)?.with_interp(Interp::parse(&s.interpolate)?);
    let direction = match s.angle {
        Some(deg) => Direction::Angle(deg),
        None => Direction::parse(&s.direction)?,
    };
    let align = Align::parse(&s.align)?;
    let banner = if let Some(path) = &s.art {
        Banner::from_art(&read_art(path)?)
    } else if s.lines {
        let parts: Vec<&str> = text
            .split('\n')
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        Banner::layout_multi(&font, &parts)?
    } else {
        Banner::layout(&font, text)?
    };
    let mode = color_mode(s, format);
    let anim = Anim::parse(&s.animate)?;

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
    let background = match &s.background {
        Some(hex) => Some(Rgb::parse(hex)?),
        None => None,
    };
    let color_by = ColorBy::parse(&s.color_by)?;
    let shadow = if s.shadow {
        Some(match &s.shadow_color {
            Some(hex) => Rgb::parse(hex)?,
            None => Rgb::new(28, 28, 34),
        })
    } else {
        None
    };
    let outline = if s.outline {
        Some(match &s.outline_color {
            Some(hex) => Rgb::parse(hex)?,
            None => Rgb::new(10, 10, 12),
        })
    } else {
        None
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
        background,
        color_by,
        shadow,
        outline,
        title: s.title.clone(),
    };

    // SVG/HTML/JSON are rendered directly from the grid, not from painted ANSI.
    if format == Format::Svg {
        let svg = if anim == Anim::Sweep {
            sigil::render::to_svg_animated(&banner, &opts, background)
        } else {
            sigil::render::to_svg(&banner, &opts, background)
        };
        return emit(s, &svg);
    }
    if format == Format::Html {
        return emit(s, &sigil::render::to_html(&banner, &opts, background));
    }
    if format == Format::Json {
        return emit(s, &sigil::render::to_json(&banner, &opts));
    }
    if format == Format::Png {
        if s.copy {
            return Err("cannot copy binary (png) to the clipboard".into());
        }
        let bytes = sigil::render::to_png(&banner, &opts, background, s.scale)?;
        return write_bytes(s.out.as_deref(), &bytes);
    }

    // Animate only for live terminal output; snippets/files/pipes render static.
    if anim.is_animated()
        && !s.copy
        && format == Format::Term
        && s.out.is_none()
        && std::io::stdout().is_terminal()
    {
        let mut out = std::io::stdout().lock();
        return animate::play(&mut out, &banner, &opts, anim, s.fps)
            .map_err(|e| format!("animation error: {e}"));
    }

    let painted = paint(&banner, &opts);
    let output = export::wrap(format, &painted);
    emit(s, &output)
}

/// Copy `content` to the clipboard when `--copy` is set, then write it out.
fn emit(s: &Settings, content: &str) -> Result<(), String> {
    if s.copy {
        copy_to_clipboard(content)?;
        eprintln!("copied to clipboard");
    }
    write_output(s.out.as_deref(), content)
}

/// Pipe `content` to the platform clipboard tool.
fn copy_to_clipboard(content: &str) -> Result<(), String> {
    use std::process::{Command, Stdio};
    let candidates: &[(&str, &[&str])] = if cfg!(target_os = "macos") {
        &[("pbcopy", &[])]
    } else if cfg!(target_os = "windows") {
        &[("clip", &[])]
    } else {
        &[
            ("wl-copy", &[]),
            ("xclip", &["-selection", "clipboard"]),
            ("xsel", &["-b", "-i"]),
        ]
    };
    for (cmd, args) in candidates {
        let child = Command::new(cmd).args(*args).stdin(Stdio::piped()).spawn();
        if let Ok(mut child) = child {
            child
                .stdin
                .take()
                .ok_or("clipboard: no stdin")?
                .write_all(content.as_bytes())
                .map_err(|e| format!("clipboard write failed: {e}"))?;
            child
                .wait()
                .map_err(|e| format!("clipboard tool failed: {e}"))?;
            return Ok(());
        }
    }
    Err("no clipboard tool found (need pbcopy / clip / wl-copy / xclip / xsel)".into())
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

/// Write binary output (e.g. PNG) to a file, or to stdout when piped. Refuses
/// to dump binary onto a terminal.
fn write_bytes(path: Option<&Path>, bytes: &[u8]) -> Result<(), String> {
    match path {
        Some(p) => {
            std::fs::write(p, bytes).map_err(|e| format!("cannot write {}: {e}", p.display()))?;
            eprintln!("wrote {}", p.display());
            Ok(())
        }
        None if std::io::stdout().is_terminal() => {
            Err("refusing to write binary to a terminal; use -o <file> or redirect".into())
        }
        None => std::io::stdout()
            .write_all(bytes)
            .map_err(|e| format!("cannot write output: {e}")),
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

/// Build the gradient from --colors, a user-defined gradient, or a built-in
/// preset (in that order of precedence).
fn resolve_gradient(s: &Settings) -> Result<Gradient, String> {
    if let Some(list) = &s.colors {
        return parse_stops(&list.split(',').map(str::to_string).collect::<Vec<_>>())
            .map_err(|e| format!("--colors: {e}"));
    }
    if let Some(stops) = s.user_gradients.get(&s.gradient) {
        return parse_stops(stops).map_err(|e| format!("gradient {:?}: {e}", s.gradient));
    }
    Gradient::preset(&s.gradient)
        .ok_or_else(|| format!("unknown gradient: {}. See `sigil gradients`.", s.gradient))
}

/// Parse a list of hex color stops into a gradient.
fn parse_stops(stops: &[String]) -> Result<Gradient, String> {
    let colors = stops
        .iter()
        .map(|c| Rgb::parse(c.trim()))
        .collect::<Result<Vec<_>, _>>()?;
    if colors.is_empty() {
        return Err("needs at least one hex stop".into());
    }
    Ok(Gradient::new(&colors))
}

fn list_gradients(mode: ColorMode, cfg: &Config) -> Result<(), String> {
    println!("Built-in gradients:\n");
    for name in Gradient::preset_names() {
        let g = Gradient::preset(name).unwrap();
        println!("  {}  {name}", swatch(&g, mode, 24));
    }
    if !cfg.gradients.is_empty() {
        println!("\nYour gradients (from config):\n");
        let mut names: Vec<&String> = cfg.gradients.keys().collect();
        names.sort();
        for name in names {
            let g =
                parse_stops(&cfg.gradients[name]).map_err(|e| format!("gradient {name:?}: {e}"))?;
            println!("  {}  {name}", swatch(&g, mode, 24));
        }
    }
    println!("\nCustom on the fly: --colors \"#ff5f6d,#ffc371\"");
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
        background: None,
        color_by: ColorBy::Banner,
        shadow: None,
        outline: None,
        title: None,
    };
    print!("{}", paint(&banner, &opts));
    Ok(())
}

/// Print a shell completion script for `shell` to stdout.
fn print_completions(shell: clap_complete::Shell) {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
}

/// Render the man page (roff) to stdout.
fn print_man() -> Result<(), String> {
    let man = clap_mangen::Man::new(Cli::command());
    let mut buf = Vec::new();
    man.render(&mut buf)
        .map_err(|e| format!("failed to render man page: {e}"))?;
    std::io::stdout()
        .write_all(&buf)
        .map_err(|e| format!("failed to write man page: {e}"))
}

/// Write (or print) the starter config file.
fn init_config(force: bool, print: bool) -> Result<(), String> {
    if print {
        print!("{}", sigil::config::STARTER);
        return Ok(());
    }
    let path = sigil::config::user_config_path()
        .ok_or("cannot determine a config directory (set HOME or XDG_CONFIG_HOME)")?;
    if path.exists() && !force {
        return Err(format!(
            "{} already exists; use --force to overwrite (or --print)",
            path.display()
        ));
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    std::fs::write(&path, sigil::config::STARTER)
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    eprintln!("wrote {}", path.display());
    Ok(())
}

fn list_themes(cfg: &Config) {
    println!("Built-in themes:\n");
    for name in themes::builtin_names() {
        print_theme(name, &themes::builtin(name).unwrap());
    }
    if !cfg.themes.is_empty() {
        println!("\nYour themes (from config):\n");
        let mut names: Vec<&String> = cfg.themes.keys().collect();
        names.sort();
        for name in names {
            print_theme(name, &cfg.themes[name]);
        }
    }
    println!("\nUse: sigil \"Text\" --theme <name>  (flags still override)");
}

fn print_theme(name: &str, t: &Theme) {
    let parts = [
        t.font.as_deref().map(|v| format!("font={v}")),
        t.gradient.as_deref().map(|v| format!("gradient={v}")),
        t.border.as_deref().map(|v| format!("border={v}")),
        t.background.as_deref().map(|v| format!("bg={v}")),
    ];
    let desc: Vec<String> = parts.into_iter().flatten().collect();
    println!("  {name:<11}  {}", desc.join("  "));
}

/// A tiny SplitMix64 PRNG — enough for picking a random font/gradient without
/// pulling in an rng crate.
struct SplitMix(u64);

impl SplitMix {
    fn new(seed: u64) -> SplitMix {
        SplitMix(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// Choose a random element from `items`.
fn choose(items: &[&str], rng: &mut SplitMix) -> String {
    let i = (rng.next() % items.len() as u64) as usize;
    items[i].to_string()
}

/// Bundled font names, for random selection.
fn font_names() -> Vec<&'static str> {
    fonts::catalog().map(|f| f.name).collect()
}

/// Seed for `--random`: the explicit `--seed`, or the current time.
fn random_seed(explicit: Option<u64>) -> u64 {
    explicit.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x5EED)
    })
}

/// Read ASCII art from a file, or stdin when the path is `-`.
fn read_art(path: &Path) -> Result<String, String> {
    if path.as_os_str() == "-" {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("failed to read stdin: {e}"))?;
        Ok(buf)
    } else {
        std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read art {}: {e}", path.display()))
    }
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
        assert_eq!(resolve_text(&args, false).unwrap(), "Hello World");
        // In --lines mode each argument becomes its own line.
        assert_eq!(resolve_text(&args, true).unwrap(), "Hello\nWorld");
    }
}
