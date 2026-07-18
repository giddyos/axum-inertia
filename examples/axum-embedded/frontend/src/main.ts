import "./style.css";

const mount = document.querySelector<HTMLElement>("#app");

if (mount) {
  const page = JSON.parse(mount.dataset.page ?? "{}");
  mount.textContent = page.props?.message ?? "Embedded Inertia frontend";
}
