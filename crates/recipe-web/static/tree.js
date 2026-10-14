(function () {
  "use strict";
  document.addEventListener("click", function (event) {
    var target = event.target;
    if (!target || !target.closest) {
      return;
    }
    var button = target.closest(".tree button[aria-controls]");
    if (!button) {
      return;
    }
    var group = document.getElementById(button.getAttribute("aria-controls"));
    if (!group) {
      return;
    }
    var expand = button.getAttribute("aria-expanded") !== "true";
    button.setAttribute("aria-expanded", expand ? "true" : "false");
    group.hidden = !expand;
  });
})();
