mod auth;
mod whoami;

use clap::{Parser, Subcommand};

/// paintbrush: a CLI for interacting with Canvas LMS, for humans and agents.
#[derive(Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Log in to a Canvas instance via browser OAuth and store credentials for future commands.
    ///
    /// Opens your default browser to Canvas's login/authorization page (falling back to
    /// printing the URL if a browser can't be opened automatically). After you approve
    /// access, your browser lands on a "Page Not Found" page at sso.canvaslms.com — copy
    /// the `code` value from that page's URL and paste it back at the prompt. The
    /// resulting access and refresh tokens are stored in your OS keychain, scoped to
    /// this domain, for future commands to use.
    Login {
        /// Canvas domain, e.g. gatech.instructure.com
        #[arg(long)]
        domain: String,
    },
    /// Print the logged-in user for a Canvas domain, using stored credentials.
    Whoami {
        /// Canvas domain, e.g. gatech.instructure.com
        #[arg(long)]
        domain: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Login { domain } => auth::login(&domain),
        Commands::Whoami { domain } => whoami::whoami(&domain),
    };

    if let Err(err) = result {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }
}
