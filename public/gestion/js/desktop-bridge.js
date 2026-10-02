/**
 * Pont MIND desktop — API distante, sans mails/messagerie, Événementiel → Production.
 * Chargé avant app.js.
 */
(function () {
  "use strict";

  function readApiUrl() {
    try {
      var q = new URLSearchParams(window.location.search || "");
      var fromQuery = (q.get("api") || "").trim();
      if (fromQuery) {
        localStorage.setItem("mind_gestion_api_url", fromQuery.replace(/\/$/, ""));
        return fromQuery.replace(/\/$/, "");
      }
    } catch (e) {
      /* ignore */
    }
    if (window.__MIND_GESTION_API_URL__) {
      return String(window.__MIND_GESTION_API_URL__).replace(/\/$/, "");
    }
    try {
      var stored = (localStorage.getItem("mind_gestion_api_url") || "").trim();
      if (stored) return stored.replace(/\/$/, "");
    } catch (e2) {
      /* ignore */
    }
    return "";
  }

  var apiUrl = readApiUrl();
  if (apiUrl) {
    window.API_URL = apiUrl;
  }

  function hideMailAndMessaging() {
    var ids = [
      "navMessagerieItem",
      "navMailsItem",
      "messageriePage",
      "mailsPage",
      "msgFloatToggle",
      "msgFloatWidget",
    ];
    ids.forEach(function (id) {
      var el = document.getElementById(id);
      if (el) el.style.display = "none";
    });
    document
      .querySelectorAll(
        '.home-tile[onclick*="openMessagingPage"], .home-tile[onclick*="openMailsPage"], [data-permission="messagerie"], [data-permission="mails"]',
      )
      .forEach(function (el) {
        el.style.display = "none";
      });
  }

  function renameEvenementielToProduction() {
    var nav = document.getElementById("navEvenementielItem");
    if (nav) {
      var label = nav.querySelector("span:last-child") || nav.querySelector("span");
      // structure: icon span + text span
      var spans = nav.querySelectorAll("span");
      if (spans.length >= 2) spans[spans.length - 1].textContent = "Production";
      else if (label) label.textContent = "Production";
    }
    document.querySelectorAll(".home-welcome-title").forEach(function (el) {
      if ((el.textContent || "").trim() === "Événementiel") {
        el.textContent = "Production";
      }
    });
    document.querySelectorAll("button, a, span, h1, h2").forEach(function (el) {
      if (!el.childElementCount && /Retour Événementiel/.test(el.textContent || "")) {
        el.textContent = (el.textContent || "").replace("Événementiel", "Production");
      }
    });
    var sideTitle = document.getElementById("sidebarTitle");
    if (sideTitle && /Gestion/i.test(sideTitle.textContent || "")) {
      /* keep brand unless empty */
    }
  }

  function showApiBannerIfNeeded() {
    if (window.API_URL && window.API_URL !== "api") return;
    if (document.getElementById("mindApiBanner")) return;
    var bar = document.createElement("div");
    bar.id = "mindApiBanner";
    bar.setAttribute(
      "style",
      "position:fixed;z-index:99999;left:0;right:0;top:0;padding:10px 14px;background:#0f6e5c;color:#fff;font:14px/1.4 Manrope,Segoe UI,sans-serif;display:flex;gap:8px;align-items:center;flex-wrap:wrap;",
    );
    bar.innerHTML =
      '<span style="flex:1 1 auto">Configure l’URL de l’API gestion (PHP), ex. https://ton-domaine/api</span>' +
      '<input id="mindApiInput" type="url" placeholder="https://…/api" style="flex:1 1 16rem;min-width:12rem;padding:6px 8px;border:0;border-radius:8px;" />' +
      '<button id="mindApiSave" type="button" style="padding:6px 12px;border:0;border-radius:8px;background:#fff;color:#0f6e5c;font-weight:700;cursor:pointer">Enregistrer</button>';
    document.documentElement.appendChild(bar);
    document.body && (document.body.style.paddingTop = "52px");
    document.getElementById("mindApiSave").onclick = function () {
      var v = (document.getElementById("mindApiInput").value || "").trim().replace(/\/$/, "");
      if (!v) return;
      localStorage.setItem("mind_gestion_api_url", v);
      window.API_URL = v;
      location.reload();
    };
  }

  function boot() {
    hideMailAndMessaging();
    renameEvenementielToProduction();
    showApiBannerIfNeeded();
    // Re-apply after SPA nav paints pages
    var obs = new MutationObserver(function () {
      hideMailAndMessaging();
      renameEvenementielToProduction();
    });
    if (document.body) {
      obs.observe(document.body, { childList: true, subtree: true });
    }
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot);
  } else {
    boot();
  }

  // Empêcher l’init messagerie float si présente
  window.__MIND_DESKTOP__ = true;
})();
