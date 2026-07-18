const e=document.querySelector("#app[data-page]"),t=e?.dataset.page?JSON.parse(e.dataset.page):{};document.querySelector("#app").textContent=t.props?.message??"Rocket + Inertia";
