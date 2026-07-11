use inertia_axum::{
    AskamaRoot, AskamaRootContext,
    askama::{self, Template},
};
use std::sync::Arc;

#[derive(Template)]
#[template(path = "app.html", askama = askama)]
pub struct AppTemplate<'a> {
    pub inertia: AskamaRootContext<'a>,
    pub app_name: &'a str,
    pub description: &'a str,
    pub locale: &'a str,
    pub social_image_url: &'a str,
}

#[derive(Clone)]
pub struct AppRoot {
    app_name: Arc<str>,
    description: Arc<str>,
    locale: Arc<str>,
    social_image_url: Arc<str>,
}

impl AppRoot {
    pub fn new(app_name: impl Into<Arc<str>>, description: impl Into<Arc<str>>) -> Self {
        Self {
            app_name: app_name.into(),
            description: description.into(),
            locale: Arc::from("en"),
            social_image_url: Arc::from("/images/social-card.png"),
        }
    }
}

impl AskamaRoot for AppRoot {
    type Template<'a> = AppTemplate<'a>;

    fn template<'a>(&'a self, inertia: AskamaRootContext<'a>) -> Self::Template<'a> {
        AppTemplate {
            inertia,
            app_name: self.app_name.as_ref(),
            description: self.description.as_ref(),
            locale: self.locale.as_ref(),
            social_image_url: self.social_image_url.as_ref(),
        }
    }
}
