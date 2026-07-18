import "./style.css";

const element = document.querySelector<HTMLElement>("#app");
if (element) {
  const page = JSON.parse(element.dataset.page ?? "{}");
  element.textContent =
    page.props?.message ?? "Embedded Actix Web Inertia frontend";
}
