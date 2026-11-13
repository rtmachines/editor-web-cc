(function () {
  "use strict";
  var forms = new WeakMap();
  document.addEventListener("htmx:beforeRequest", function (event) {
    document.querySelectorAll("#error-panel [data-transport-error]").forEach(function (lost) { lost.remove(); });
    var form = event.detail.elt.closest("form");
    if (!form) return;
    forms.set(event.detail.xhr, form);
    form.openRequests = (form.openRequests || 0) + 1;
    form.setAttribute("aria-busy", "true");
    form.querySelectorAll("textarea, input:not([type=hidden], [type=checkbox], [type=radio], [type=submit], [type=button], [type=reset])").forEach(function (control) {
      if (control.readOnly) return;
      control.readOnly = true;
      control.dataset.requestLock = "";
    });
  });
  function unlock(event) {
    var form = forms.get(event.detail.xhr);
    forms.delete(event.detail.xhr);
    if (!form) return;
    form.openRequests = Math.max(0, form.openRequests - 1);
    if (form.openRequests > 0) return;
    form.removeAttribute("aria-busy");
    form.querySelectorAll("[data-request-lock]").forEach(function (control) { control.readOnly = false; delete control.dataset.requestLock; });
  }
  ["htmx:afterRequest", "htmx:onLoadError"].forEach(function (name) { document.addEventListener(name, unlock); });
  ["keydown", "mousedown", "click"].forEach(function (name) {
    document.addEventListener(name, function (event) {
      if (event.key === "Tab" || (event.type === "keydown" && (event.ctrlKey || event.altKey || event.metaKey || /^F\d{1,2}$/.test(event.key))) || !event.target.matches("select, input[type=checkbox], input[type=radio]")) return;
      if (event.target.closest('form[aria-busy="true"]')) event.preventDefault();
    }, true);
  });
  function showLostAnswer() {
    var panel = document.getElementById("error-panel");
    panel.replaceChildren(document.getElementById("transport-error").content.cloneNode(true));
    document.getElementById("toast").replaceChildren();
    panel.scrollIntoView({ block: "start" });
  }
  ["htmx:sendError", "htmx:timeout"].forEach(function (name) { document.addEventListener(name, showLostAnswer); });
})();
