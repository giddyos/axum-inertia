#[derive(inertia_actix::InertiaPage)]
#[inertia(component = "Home")]
struct Home {
    message: String,
}

fn main() {
    let page = Home {
        message: "Actix adapter".to_owned(),
    };
    let _: inertia_actix::PendingPage = inertia_actix::PendingPage::typed(page);
}
