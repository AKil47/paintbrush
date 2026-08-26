mod announcement;
mod assignment;
mod auth;
mod course;
mod page;
mod profile;
mod whoami;

use clap::{Args, Parser, Subcommand};

/// paintbrush: a CLI for interacting with Canvas LMS, for humans and agents.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Profile to use for this command. Defaults to the default profile —
    /// see `paintbrush profile list` and `paintbrush profile default`.
    #[arg(long, global = true)]
    profile: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

/// A required course ID, shared by every command that targets a
/// course-scoped resource — Canvas's own API nests these resources under
/// `/courses/:course_id/...`, so there's no ambient "current course" to
/// default to.
#[derive(Args)]
struct CourseScope {
    /// Course ID (see `paintbrush course list`)
    #[arg(long)]
    course: u64,
}

#[derive(Subcommand)]
enum Commands {
    /// Log in to a Canvas instance via browser OAuth and store credentials under a profile.
    ///
    /// Opens your default browser to Canvas's login/authorization page (falling back to
    /// printing the URL if a browser can't be opened automatically). After you approve
    /// access, your browser lands on a "Page Not Found" page at sso.canvaslms.com — copy
    /// the `code` value from that page's URL and paste it back at the prompt. The
    /// resulting access and refresh tokens are stored in your OS keychain under this
    /// profile (name given by `--profile`, or the domain itself if omitted). The first
    /// profile ever created becomes the default used when `--profile` isn't passed.
    Login {
        /// Canvas domain, e.g. gatech.instructure.com
        #[arg(long)]
        domain: String,
    },
    /// Print the logged-in user for the selected profile.
    Whoami,
    /// Course-related commands.
    Course {
        #[command(subcommand)]
        command: CourseCommands,
    },
    /// Assignment-related commands.
    Assignment {
        #[command(subcommand)]
        command: AssignmentCommands,
    },
    /// Announcement-related commands.
    Announcement {
        #[command(subcommand)]
        command: AnnouncementCommands,
    },
    /// Commands for a Canvas page, addressed by its full URL — e.g. a quiz,
    /// wiki page, or syllabus. Unlike other resources, a page isn't looked up
    /// by numeric ID; you address it with the URL you'd open in a browser.
    Page {
        /// Full Canvas URL of the page, e.g. https://gatech.instructure.com/courses/1234/quizzes/5678
        url: String,
        #[command(subcommand)]
        command: PageCommands,
    },
    /// Manage stored login profiles (add profiles via `paintbrush login`).
    Profile {
        #[command(subcommand)]
        command: ProfileCommands,
    },
}

#[derive(Subcommand)]
enum CourseCommands {
    /// List the logged-in user's courses for the selected profile.
    ///
    /// Prints one course per line as `id\tcourse_code\tname`. The `id` is
    /// what course-scoped commands (e.g. `assignment list`) take via `--course`.
    List,
    /// Show full details for a course, for the selected profile.
    View {
        /// Course ID (see `paintbrush course list`)
        id: u64,
        /// Open the course in your browser instead of printing details.
        #[arg(long)]
        web: bool,
    },
}

#[derive(Subcommand)]
enum AssignmentCommands {
    /// List the published assignments in a course, for the selected profile.
    ///
    /// Prints one assignment per line as `id\tdue_at\tlock_state\tpoints_possible\tname`.
    /// `lock_state` is `locked` or `unlocked`, e.g. an assignment locked until a future
    /// unlock date. Unpublished assignments are omitted.
    List {
        #[command(flatten)]
        scope: CourseScope,
    },
    /// Show full details for an assignment, for the selected profile.
    View {
        /// Assignment ID (see `paintbrush assignment list`)
        id: u64,
        #[command(flatten)]
        scope: CourseScope,
        /// Open the assignment in your browser instead of printing details.
        #[arg(long)]
        web: bool,
    },
}

#[derive(Subcommand)]
enum AnnouncementCommands {
    /// List the published announcements in a course, for the selected profile.
    ///
    /// Prints one announcement per line as `id\tposted_at\tlock_state\ttitle`.
    /// `lock_state` is `locked` or `unlocked`, e.g. an announcement closed for comments.
    /// Uses Canvas's default date range (announcements posted in roughly the last 14
    /// days). Unpublished announcements are omitted.
    List {
        #[command(flatten)]
        scope: CourseScope,
    },
    /// Show full details for an announcement, for the selected profile.
    View {
        /// Announcement ID (see `paintbrush announcement list`)
        id: u64,
        #[command(flatten)]
        scope: CourseScope,
        /// Open the announcement in your browser instead of printing details.
        #[arg(long)]
        web: bool,
    },
}

#[derive(Subcommand)]
enum PageCommands {
    /// Print the page's rendered HTML, or open it in the browser if `--web`
    /// is set, for the selected profile.
    ///
    /// Printing goes through Canvas's web session rather than `/api/v1` —
    /// needed for content (e.g. quizzes) that Canvas only serves as rendered
    /// HTML. Reuses a stored session cookie when possible; otherwise (first
    /// use, or once Canvas no longer honors it) transparently establishes a
    /// fresh one via Canvas's `session_token` endpoint and stores it for next
    /// time.
    View {
        /// Open the page in your browser instead of printing its HTML.
        #[arg(long)]
        web: bool,
    },
}

#[derive(Subcommand)]
enum ProfileCommands {
    /// List configured profiles; `*` marks the default.
    List,
    /// Remove a profile and its stored credentials.
    Remove {
        name: String,
    },
    /// Set the default profile used when `--profile` isn't passed.
    Default {
        name: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let selected_profile = cli.profile.as_deref();

    let result = match cli.command {
        Commands::Login { domain } => auth::login(selected_profile, &domain),
        Commands::Whoami => whoami::whoami(selected_profile),
        Commands::Course { command } => match command {
            CourseCommands::List => course::list(selected_profile),
            CourseCommands::View { id, web } => course::view(selected_profile, id, web),
        },
        Commands::Assignment { command } => match command {
            AssignmentCommands::List { scope } => assignment::list(selected_profile, scope.course),
            AssignmentCommands::View { id, scope, web } => {
                assignment::view(selected_profile, scope.course, id, web)
            }
        },
        Commands::Announcement { command } => match command {
            AnnouncementCommands::List { scope } => announcement::list(selected_profile, scope.course),
            AnnouncementCommands::View { id, scope, web } => {
                announcement::view(selected_profile, scope.course, id, web)
            }
        },
        Commands::Page { url, command } => match command {
            PageCommands::View { web } => page::view(selected_profile, &url, web),
        },
        Commands::Profile { command } => match command {
            ProfileCommands::List => profile::list(),
            ProfileCommands::Remove { name } => profile::remove(&name),
            ProfileCommands::Default { name } => profile::set_default(&name),
        },
    };

    if let Err(err) = result {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }
}
