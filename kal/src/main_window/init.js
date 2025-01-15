(function () {
  window.KAL.config = __RAW_config__;

  let custom_css = __TEMPLATE_custom_css__;
  if (custom_css) {
    window.addEventListener("DOMContentLoaded", () => {
      const style = document.createElement("style");
      style.textContent = custom_css;
      const head = document.head ?? document.querySelector("head") ?? document.body;
      head.appendChild(style);
    });
  }
})();
