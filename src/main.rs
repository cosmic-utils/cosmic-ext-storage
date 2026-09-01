// SPDX-License-Identifier: GPL-3.0-only

fn main() -> cosmic::iced::Result {
    let request = match cosmic_ext_storage::RuntimeRequest::parse(std::env::args().skip(1)) {
        Ok(request) => request,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    let runtime = match cosmic_ext_storage::AppRuntime::from_request(request) {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };

    let config = cosmic_ext_storage::config::Config::load(cosmic_ext_storage::app::APP_ID);
    cosmic_ext_storage::logging::init(&config);

    // Get the system's preferred languages.
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    // Enable localizations to be applied.
    cosmic_ext_storage::i18n::init(&requested_languages);

    // Settings for configuring the application window and iced runtime.
    let settings = cosmic::app::Settings::default().size_limits(
        cosmic::iced::Limits::NONE
            .min_width(360.0)
            .min_height(180.0),
    );

    // Starts the application's event loop with `()` as the application's flags.
    cosmic::app::run::<cosmic_ext_storage::AppModel>(settings, runtime)
}
