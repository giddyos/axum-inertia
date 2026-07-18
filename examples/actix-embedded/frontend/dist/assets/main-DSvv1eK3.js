const e=document.querySelector("#app");if(e){const t=JSON.parse(e.dataset.page??"{}");e.textContent=t.props?.message??"Embedded Actix Web Inertia frontend"}
