(function () {
  "use strict";
  document.addEventListener("rowVersion", function (event) {
    var value = event.detail && event.detail.value;
    if (!Number.isSafeInteger(value) || value < 0) {
      return;
    }
    var text = String(value);
    document.querySelectorAll('input[name="row_version"]').forEach(function (input) {
      input.value = text;
    });
  });
})();
