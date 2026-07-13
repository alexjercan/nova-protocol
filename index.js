/*
 * ATTENTION: The "eval" devtool has been used (maybe by default in mode: "development").
 * This devtool is neither made for production nor for readable output files.
 * It uses "eval()" calls to create a separate source file in the browser devtools.
 * If you are trying to read the output file, select a different devtool (https://webpack.js.org/configuration/devtool/)
 * or disable the default devtool with "devtool: false".
 * If you are looking for production-ready output files, see mode: "production" (https://webpack.js.org/configuration/mode/).
 */
/******/ (() => { // webpackBootstrap
/******/ 	"use strict";
/******/ 	var __webpack_modules__ = ({

/***/ "./node_modules/css-loader/dist/cjs.js!./node_modules/postcss-loader/dist/cjs.js!./src/style.css"
/*!*******************************************************************************************************!*\
  !*** ./node_modules/css-loader/dist/cjs.js!./node_modules/postcss-loader/dist/cjs.js!./src/style.css ***!
  \*******************************************************************************************************/
(module, __webpack_exports__, __webpack_require__) {

eval("{__webpack_require__.r(__webpack_exports__);\n/* harmony export */ __webpack_require__.d(__webpack_exports__, {\n/* harmony export */   \"default\": () => (__WEBPACK_DEFAULT_EXPORT__)\n/* harmony export */ });\n/* harmony import */ var _node_modules_css_loader_dist_runtime_noSourceMaps_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ../node_modules/css-loader/dist/runtime/noSourceMaps.js */ \"./node_modules/css-loader/dist/runtime/noSourceMaps.js\");\n/* harmony import */ var _node_modules_css_loader_dist_runtime_noSourceMaps_js__WEBPACK_IMPORTED_MODULE_0___default = /*#__PURE__*/__webpack_require__.n(_node_modules_css_loader_dist_runtime_noSourceMaps_js__WEBPACK_IMPORTED_MODULE_0__);\n/* harmony import */ var _node_modules_css_loader_dist_runtime_api_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ../node_modules/css-loader/dist/runtime/api.js */ \"./node_modules/css-loader/dist/runtime/api.js\");\n/* harmony import */ var _node_modules_css_loader_dist_runtime_api_js__WEBPACK_IMPORTED_MODULE_1___default = /*#__PURE__*/__webpack_require__.n(_node_modules_css_loader_dist_runtime_api_js__WEBPACK_IMPORTED_MODULE_1__);\n// Imports\n\n\nvar ___CSS_LOADER_EXPORT___ = _node_modules_css_loader_dist_runtime_api_js__WEBPACK_IMPORTED_MODULE_1___default()((_node_modules_css_loader_dist_runtime_noSourceMaps_js__WEBPACK_IMPORTED_MODULE_0___default()));\n// Module\n___CSS_LOADER_EXPORT___.push([module.id, `/*! tailwindcss v4.3.2 | MIT License | https://tailwindcss.com */\n@layer properties;\n.visible {\n  visibility: visible;\n}\n.fixed {\n  position: fixed;\n}\n.relative {\n  position: relative;\n}\n.static {\n  position: static;\n}\n.container {\n  width: 100%;\n}\n.grid {\n  display: grid;\n}\n.transform {\n  transform: var(--tw-rotate-x,) var(--tw-rotate-y,) var(--tw-rotate-z,) var(--tw-skew-x,) var(--tw-skew-y,);\n}\n.outline {\n  outline-style: var(--tw-outline-style);\n  outline-width: 1px;\n}\n.invert {\n  --tw-invert: invert(100%);\n  filter: var(--tw-blur,) var(--tw-brightness,) var(--tw-contrast,) var(--tw-grayscale,) var(--tw-hue-rotate,) var(--tw-invert,) var(--tw-saturate,) var(--tw-sepia,) var(--tw-drop-shadow,);\n}\n.filter {\n  filter: var(--tw-blur,) var(--tw-brightness,) var(--tw-contrast,) var(--tw-grayscale,) var(--tw-hue-rotate,) var(--tw-invert,) var(--tw-saturate,) var(--tw-sepia,) var(--tw-drop-shadow,);\n}\n:root {\n  --space-0: #070a14;\n  --space-1: #0b0f1c;\n  --panel: #141a2e;\n  --panel-2: #1a2138;\n  --border: #233052;\n  --cyan: #5cc8ff;\n  --cyan-bright: #8fe0ff;\n  --cyan-deep: #2a9fd6;\n  --amber: #ffb877;\n  --amber-horizon: #ff7a3c;\n  --text: #e8eefc;\n  --text-muted: #8b95b0;\n  --font-display: \"Rajdhani\", \"Segoe UI\", Tahoma, sans-serif;\n  --font-body: \"Inter\", -apple-system, BlinkMacSystemFont, \"Segoe UI\", sans-serif;\n  --font-mono: \"JetBrains Mono\", \"SFMono-Regular\", Consolas, monospace;\n  --shadow-glow-cyan: 0 0 24px rgba(92, 200, 255, 0.35);\n  --shadow-panel: 0 8px 32px rgba(0, 0, 0, 0.45);\n}\n* {\n  box-sizing: border-box;\n}\nhtml {\n  scroll-behavior: smooth;\n}\nbody {\n  margin: 0;\n  min-height: 100vh;\n  display: flex;\n  flex-direction: column;\n  color: var(--text);\n  font-family: var(--font-body);\n  line-height: 1.6;\n  background-color: var(--space-1);\n  background-image: radial-gradient(\n            1px 1px at 20% 30%,\n            rgba(232, 238, 252, 0.5),\n            transparent\n        ),\n        radial-gradient(\n            1px 1px at 75% 20%,\n            rgba(232, 238, 252, 0.35),\n            transparent\n        ),\n        radial-gradient(\n            1px 1px at 45% 65%,\n            rgba(232, 238, 252, 0.4),\n            transparent\n        ),\n        radial-gradient(\n            1.5px 1.5px at 85% 75%,\n            rgba(232, 238, 252, 0.3),\n            transparent\n        ),\n        radial-gradient(\n            1px 1px at 10% 85%,\n            rgba(232, 238, 252, 0.35),\n            transparent\n        );\n  background-attachment: fixed;\n}\nh1,\nh2,\nh3 {\n  font-family: var(--font-display);\n  font-weight: 700;\n  letter-spacing: 0.02em;\n  line-height: 1.15;\n  margin: 0 0 0.5em;\n}\na {\n  color: var(--cyan);\n  text-decoration: none;\n  transition: color 0.15s ease;\n}\na:hover {\n  color: var(--cyan-bright);\n}\n.glow-cyan {\n  color: var(--cyan-bright);\n  text-shadow: 0 0 8px rgba(92, 200, 255, 0.75),\n        0 0 22px rgba(92, 200, 255, 0.45);\n}\n.glow-amber {\n  color: var(--amber);\n  text-shadow: 0 0 8px rgba(255, 122, 60, 0.6),\n        0 0 22px rgba(255, 122, 60, 0.35);\n}\n.site-header {\n  position: sticky;\n  top: 0;\n  z-index: 50;\n  display: flex;\n  align-items: center;\n  justify-content: space-between;\n  gap: 16px;\n  padding: 14px 24px;\n  background: rgba(11, 15, 28, 0.82);\n  backdrop-filter: blur(10px);\n  border-bottom: 1px solid var(--border);\n}\n.site-header__brand {\n  font-family: var(--font-display);\n  font-weight: 700;\n  font-size: 1.15rem;\n  letter-spacing: 0.14em;\n  text-transform: uppercase;\n  color: var(--text);\n}\n.site-header__brand b {\n  color: var(--cyan-bright);\n  text-shadow: 0 0 10px rgba(92, 200, 255, 0.6);\n}\n.site-nav {\n  display: flex;\n  align-items: center;\n  gap: 4px;\n  flex-wrap: wrap;\n}\n.site-nav a {\n  padding: 8px 12px;\n  border-radius: 8px;\n  color: var(--text-muted);\n  font-size: 0.9rem;\n  font-weight: 600;\n  letter-spacing: 0.02em;\n}\n.site-nav a:hover {\n  color: var(--text);\n  background: rgba(92, 200, 255, 0.08);\n}\n.site-nav a.is-cta {\n  color: var(--space-0);\n  background: linear-gradient(135deg, var(--cyan-bright), var(--cyan-deep));\n  box-shadow: var(--shadow-glow-cyan);\n}\n.site-nav a.is-cta:hover {\n  color: var(--space-0);\n  filter: brightness(1.08);\n}\nmain {\n  flex: 1 0 auto;\n  width: 100%;\n}\n.container {\n  width: 100%;\n  max-width: 1080px;\n  margin: 0 auto;\n  padding: 0 24px;\n}\n.section {\n  padding: 72px 0;\n}\n.section__eyebrow {\n  font-family: var(--font-mono);\n  font-size: 0.78rem;\n  letter-spacing: 0.28em;\n  text-transform: uppercase;\n  color: var(--cyan);\n  margin-bottom: 10px;\n}\n.section__title {\n  font-size: 2rem;\n  margin-bottom: 12px;\n}\n.section__lead {\n  color: var(--text-muted);\n  max-width: 60ch;\n  margin: 0 0 32px;\n}\n.hero {\n  position: relative;\n  overflow: hidden;\n  text-align: center;\n  padding: 88px 24px 96px;\n}\n.hero::after {\n  content: \"\";\n  position: absolute;\n  left: 50%;\n  bottom: -55%;\n  width: 140%;\n  height: 100%;\n  transform: translateX(-50%);\n  background: radial-gradient(\n        ellipse at center,\n        rgba(255, 122, 60, 0.35) 0%,\n        rgba(255, 122, 60, 0.08) 35%,\n        transparent 65%\n    );\n  pointer-events: none;\n  z-index: 0;\n}\n.hero > * {\n  position: relative;\n  z-index: 1;\n}\n.hero__art {\n  display: block;\n  width: min(560px, 90%);\n  height: auto;\n  aspect-ratio: 3 / 2;\n  margin: 0 auto 28px;\n  border-radius: 16px;\n  box-shadow: 0 0 0 1px var(--border),\n        var(--shadow-panel),\n        0 0 60px rgba(92, 200, 255, 0.15);\n}\n.hero__tagline {\n  font-size: 1.2rem;\n  color: var(--text-muted);\n  max-width: 56ch;\n  margin: 0 auto 32px;\n}\n.hero__cta {\n  display: flex;\n  gap: 14px;\n  justify-content: center;\n  flex-wrap: wrap;\n}\n.btn {\n  display: inline-flex;\n  align-items: center;\n  gap: 8px;\n  padding: 13px 26px;\n  border-radius: 10px;\n  font-family: var(--font-display);\n  font-weight: 700;\n  font-size: 1.02rem;\n  letter-spacing: 0.06em;\n  text-transform: uppercase;\n  cursor: pointer;\n  border: 1px solid transparent;\n  transition: transform 0.12s ease,\n        filter 0.15s ease,\n        background 0.15s ease;\n}\n.btn:hover {\n  transform: translateY(-2px);\n}\n.btn--primary {\n  color: var(--space-0);\n  background: linear-gradient(135deg, var(--cyan-bright), var(--cyan-deep));\n  box-shadow: var(--shadow-glow-cyan);\n}\n.btn--primary:hover {\n  color: var(--space-0);\n  filter: brightness(1.08);\n}\n.btn--ghost {\n  color: var(--amber);\n  background: transparent;\n  border-color: rgba(255, 184, 119, 0.5);\n}\n.btn--ghost:hover {\n  color: var(--amber);\n  background: rgba(255, 122, 60, 0.1);\n  border-color: var(--amber);\n}\n.grid {\n  display: grid;\n  gap: 20px;\n}\n.grid--3 {\n  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));\n}\n.grid--2 {\n  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));\n}\n.card {\n  background: var(--panel);\n  border: 1px solid var(--border);\n  border-radius: 14px;\n  padding: 24px;\n  box-shadow: var(--shadow-panel);\n  transition: transform 0.15s ease,\n        border-color 0.15s ease;\n}\n.card:hover {\n  transform: translateY(-3px);\n  border-color: rgba(92, 200, 255, 0.5);\n}\n.card__icon {\n  font-size: 1.6rem;\n  margin-bottom: 10px;\n}\n.card__title {\n  font-size: 1.2rem;\n  margin-bottom: 6px;\n  color: var(--text);\n}\n.card__body {\n  color: var(--text-muted);\n  font-size: 0.96rem;\n  margin: 0;\n}\n.card--link {\n  display: block;\n  color: inherit;\n}\n.card--link:hover .card__title {\n  color: var(--cyan-bright);\n}\n.prose {\n  max-width: 74ch;\n  margin: 0 auto;\n  padding: 56px 24px 72px;\n}\n.prose h1 {\n  font-size: 2.2rem;\n}\n.prose h2 {\n  font-size: 1.5rem;\n  margin-top: 2em;\n  padding-top: 0.4em;\n  border-top: 1px solid var(--border);\n  color: var(--cyan-bright);\n}\n.prose h3 {\n  font-size: 1.15rem;\n  margin-top: 1.6em;\n  color: var(--amber);\n}\n.prose p,\n.prose li {\n  color: var(--text);\n}\n.prose a {\n  text-decoration: underline;\n  text-underline-offset: 3px;\n}\n.prose code {\n  font-family: var(--font-mono);\n  font-size: 0.88em;\n  background: var(--panel-2);\n  border: 1px solid var(--border);\n  border-radius: 5px;\n  padding: 1px 6px;\n  color: var(--cyan-bright);\n}\n.prose kbd {\n  font-family: var(--font-mono);\n  font-size: 0.82em;\n  background: var(--panel-2);\n  border: 1px solid var(--border);\n  border-bottom-width: 3px;\n  border-radius: 6px;\n  padding: 2px 8px;\n  color: var(--text);\n  white-space: nowrap;\n}\n.prose blockquote {\n  margin: 1.4em 0;\n  padding: 4px 18px;\n  border-left: 3px solid var(--cyan-deep);\n  color: var(--text-muted);\n  background: rgba(92, 200, 255, 0.05);\n  border-radius: 0 8px 8px 0;\n}\n.prose__meta {\n  color: var(--text-muted);\n  font-family: var(--font-mono);\n  font-size: 0.82rem;\n  letter-spacing: 0.05em;\n  margin-bottom: 1.5em;\n}\n.video-embed {\n  position: relative;\n  width: 100%;\n  aspect-ratio: 16 / 9;\n  margin: 1.6em 0;\n  border-radius: 12px;\n  overflow: hidden;\n  border: 1px solid var(--border);\n  box-shadow: var(--shadow-panel),\n        0 0 40px rgba(92, 200, 255, 0.12);\n}\n.video-embed iframe {\n  position: absolute;\n  inset: 0;\n  width: 100%;\n  height: 100%;\n  border: 0;\n}\n.video-embed__caption {\n  display: block;\n  margin-top: -0.6em;\n  margin-bottom: 1.8em;\n  color: var(--text-muted);\n  font-family: var(--font-mono);\n  font-size: 0.8rem;\n  letter-spacing: 0.04em;\n  text-align: center;\n}\n.controls {\n  width: 100%;\n  border-collapse: collapse;\n  margin: 1.2em 0;\n}\n.controls td {\n  padding: 9px 12px;\n  border-bottom: 1px solid var(--border);\n  vertical-align: top;\n}\n.controls td:first-child {\n  width: 40%;\n  white-space: nowrap;\n}\n.controls tr:last-child td {\n  border-bottom: none;\n}\n.post-list {\n  list-style: none;\n  padding: 0;\n  margin: 0;\n  display: grid;\n  gap: 18px;\n}\n.post-list__date {\n  display: block;\n  font-family: var(--font-mono);\n  font-size: 0.78rem;\n  letter-spacing: 0.08em;\n  color: var(--cyan);\n  margin-bottom: 6px;\n}\n.post-list__excerpt {\n  color: var(--text-muted);\n  margin: 6px 0 0;\n}\n.site-footer {\n  flex-shrink: 0;\n  display: flex;\n  align-items: center;\n  justify-content: space-between;\n  gap: 16px;\n  flex-wrap: wrap;\n  padding: 22px 24px;\n  margin-top: 40px;\n  border-top: 1px solid var(--border);\n  background: var(--space-0);\n  color: var(--text-muted);\n  font-size: 0.86rem;\n}\n.site-footer nav {\n  display: flex;\n  gap: 18px;\n  flex-wrap: wrap;\n}\n.site-footer a {\n  color: var(--text-muted);\n}\n.site-footer a:hover {\n  color: var(--cyan-bright);\n}\n@media (max-width: 640px) {\n  .section {\n    padding: 52px 0;\n  }\n  .section__title {\n    font-size: 1.6rem;\n  }\n  .hero {\n    padding: 56px 20px 68px;\n  }\n  .controls td:first-child {\n    width: 45%;\n  }\n}\n@property --tw-rotate-x {\n  syntax: \"*\";\n  inherits: false;\n}\n@property --tw-rotate-y {\n  syntax: \"*\";\n  inherits: false;\n}\n@property --tw-rotate-z {\n  syntax: \"*\";\n  inherits: false;\n}\n@property --tw-skew-x {\n  syntax: \"*\";\n  inherits: false;\n}\n@property --tw-skew-y {\n  syntax: \"*\";\n  inherits: false;\n}\n@property --tw-outline-style {\n  syntax: \"*\";\n  inherits: false;\n  initial-value: solid;\n}\n@property --tw-blur {\n  syntax: \"*\";\n  inherits: false;\n}\n@property --tw-brightness {\n  syntax: \"*\";\n  inherits: false;\n}\n@property --tw-contrast {\n  syntax: \"*\";\n  inherits: false;\n}\n@property --tw-grayscale {\n  syntax: \"*\";\n  inherits: false;\n}\n@property --tw-hue-rotate {\n  syntax: \"*\";\n  inherits: false;\n}\n@property --tw-invert {\n  syntax: \"*\";\n  inherits: false;\n}\n@property --tw-opacity {\n  syntax: \"*\";\n  inherits: false;\n}\n@property --tw-saturate {\n  syntax: \"*\";\n  inherits: false;\n}\n@property --tw-sepia {\n  syntax: \"*\";\n  inherits: false;\n}\n@property --tw-drop-shadow {\n  syntax: \"*\";\n  inherits: false;\n}\n@property --tw-drop-shadow-color {\n  syntax: \"*\";\n  inherits: false;\n}\n@property --tw-drop-shadow-alpha {\n  syntax: \"<percentage>\";\n  inherits: false;\n  initial-value: 100%;\n}\n@property --tw-drop-shadow-size {\n  syntax: \"*\";\n  inherits: false;\n}\n@layer properties {\n  @supports ((-webkit-hyphens: none) and (not (margin-trim: inline))) or ((-moz-orient: inline) and (not (color:rgb(from red r g b)))) {\n    *, ::before, ::after, ::backdrop {\n      --tw-rotate-x: initial;\n      --tw-rotate-y: initial;\n      --tw-rotate-z: initial;\n      --tw-skew-x: initial;\n      --tw-skew-y: initial;\n      --tw-outline-style: solid;\n      --tw-blur: initial;\n      --tw-brightness: initial;\n      --tw-contrast: initial;\n      --tw-grayscale: initial;\n      --tw-hue-rotate: initial;\n      --tw-invert: initial;\n      --tw-opacity: initial;\n      --tw-saturate: initial;\n      --tw-sepia: initial;\n      --tw-drop-shadow: initial;\n      --tw-drop-shadow-color: initial;\n      --tw-drop-shadow-alpha: 100%;\n      --tw-drop-shadow-size: initial;\n    }\n  }\n}\n`, \"\"]);\n// Exports\n/* harmony default export */ const __WEBPACK_DEFAULT_EXPORT__ = (___CSS_LOADER_EXPORT___);\n\n\n//# sourceURL=webpack://nova-protocol-web/./src/style.css?./node_modules/css-loader/dist/cjs.js!./node_modules/postcss-loader/dist/cjs.js\n}");

/***/ },

/***/ "./node_modules/css-loader/dist/runtime/api.js"
/*!*****************************************************!*\
  !*** ./node_modules/css-loader/dist/runtime/api.js ***!
  \*****************************************************/
(module) {

eval("{\n\n/*\n  MIT License http://www.opensource.org/licenses/mit-license.php\n  Author Tobias Koppers @sokra\n*/\nmodule.exports = function (cssWithMappingToString) {\n  var list = [];\n\n  // return the list of modules as css string\n  list.toString = function toString() {\n    return this.map(function (item) {\n      var content = \"\";\n      var needLayer = typeof item[5] !== \"undefined\";\n      if (item[4]) {\n        content += \"@supports (\".concat(item[4], \") {\");\n      }\n      if (item[2]) {\n        content += \"@media \".concat(item[2], \" {\");\n      }\n      if (needLayer) {\n        content += \"@layer\".concat(item[5].length > 0 ? \" \".concat(item[5]) : \"\", \" {\");\n      }\n      content += cssWithMappingToString(item);\n      if (needLayer) {\n        content += \"}\";\n      }\n      if (item[2]) {\n        content += \"}\";\n      }\n      if (item[4]) {\n        content += \"}\";\n      }\n      return content;\n    }).join(\"\");\n  };\n\n  // import a list of modules into the list\n  list.i = function i(modules, media, dedupe, supports, layer) {\n    if (typeof modules === \"string\") {\n      modules = [[null, modules, undefined]];\n    }\n    var alreadyImportedModules = {};\n    if (dedupe) {\n      for (var k = 0; k < this.length; k++) {\n        var id = this[k][0];\n        if (id != null) {\n          alreadyImportedModules[id] = true;\n        }\n      }\n    }\n    for (var _k = 0; _k < modules.length; _k++) {\n      var item = [].concat(modules[_k]);\n      if (dedupe && alreadyImportedModules[item[0]]) {\n        continue;\n      }\n      if (typeof layer !== \"undefined\") {\n        if (typeof item[5] === \"undefined\") {\n          item[5] = layer;\n        } else {\n          item[1] = \"@layer\".concat(item[5].length > 0 ? \" \".concat(item[5]) : \"\", \" {\").concat(item[1], \"}\");\n          item[5] = layer;\n        }\n      }\n      if (media) {\n        if (!item[2]) {\n          item[2] = media;\n        } else {\n          item[1] = \"@media \".concat(item[2], \" {\").concat(item[1], \"}\");\n          item[2] = media;\n        }\n      }\n      if (supports) {\n        if (!item[4]) {\n          item[4] = \"\".concat(supports);\n        } else {\n          item[1] = \"@supports (\".concat(item[4], \") {\").concat(item[1], \"}\");\n          item[4] = supports;\n        }\n      }\n      list.push(item);\n    }\n  };\n  return list;\n};\n\n//# sourceURL=webpack://nova-protocol-web/./node_modules/css-loader/dist/runtime/api.js?\n}");

/***/ },

/***/ "./node_modules/css-loader/dist/runtime/noSourceMaps.js"
/*!**************************************************************!*\
  !*** ./node_modules/css-loader/dist/runtime/noSourceMaps.js ***!
  \**************************************************************/
(module) {

eval("{\n\nmodule.exports = function (i) {\n  return i[1];\n};\n\n//# sourceURL=webpack://nova-protocol-web/./node_modules/css-loader/dist/runtime/noSourceMaps.js?\n}");

/***/ },

/***/ "./src/style.css"
/*!***********************!*\
  !*** ./src/style.css ***!
  \***********************/
(__unused_webpack_module, __webpack_exports__, __webpack_require__) {

eval("{__webpack_require__.r(__webpack_exports__);\n/* harmony export */ __webpack_require__.d(__webpack_exports__, {\n/* harmony export */   \"default\": () => (__WEBPACK_DEFAULT_EXPORT__)\n/* harmony export */ });\n/* harmony import */ var _node_modules_style_loader_dist_runtime_injectStylesIntoStyleTag_js__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! !../node_modules/style-loader/dist/runtime/injectStylesIntoStyleTag.js */ \"./node_modules/style-loader/dist/runtime/injectStylesIntoStyleTag.js\");\n/* harmony import */ var _node_modules_style_loader_dist_runtime_injectStylesIntoStyleTag_js__WEBPACK_IMPORTED_MODULE_0___default = /*#__PURE__*/__webpack_require__.n(_node_modules_style_loader_dist_runtime_injectStylesIntoStyleTag_js__WEBPACK_IMPORTED_MODULE_0__);\n/* harmony import */ var _node_modules_style_loader_dist_runtime_styleDomAPI_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! !../node_modules/style-loader/dist/runtime/styleDomAPI.js */ \"./node_modules/style-loader/dist/runtime/styleDomAPI.js\");\n/* harmony import */ var _node_modules_style_loader_dist_runtime_styleDomAPI_js__WEBPACK_IMPORTED_MODULE_1___default = /*#__PURE__*/__webpack_require__.n(_node_modules_style_loader_dist_runtime_styleDomAPI_js__WEBPACK_IMPORTED_MODULE_1__);\n/* harmony import */ var _node_modules_style_loader_dist_runtime_insertBySelector_js__WEBPACK_IMPORTED_MODULE_2__ = __webpack_require__(/*! !../node_modules/style-loader/dist/runtime/insertBySelector.js */ \"./node_modules/style-loader/dist/runtime/insertBySelector.js\");\n/* harmony import */ var _node_modules_style_loader_dist_runtime_insertBySelector_js__WEBPACK_IMPORTED_MODULE_2___default = /*#__PURE__*/__webpack_require__.n(_node_modules_style_loader_dist_runtime_insertBySelector_js__WEBPACK_IMPORTED_MODULE_2__);\n/* harmony import */ var _node_modules_style_loader_dist_runtime_setAttributesWithoutAttributes_js__WEBPACK_IMPORTED_MODULE_3__ = __webpack_require__(/*! !../node_modules/style-loader/dist/runtime/setAttributesWithoutAttributes.js */ \"./node_modules/style-loader/dist/runtime/setAttributesWithoutAttributes.js\");\n/* harmony import */ var _node_modules_style_loader_dist_runtime_setAttributesWithoutAttributes_js__WEBPACK_IMPORTED_MODULE_3___default = /*#__PURE__*/__webpack_require__.n(_node_modules_style_loader_dist_runtime_setAttributesWithoutAttributes_js__WEBPACK_IMPORTED_MODULE_3__);\n/* harmony import */ var _node_modules_style_loader_dist_runtime_insertStyleElement_js__WEBPACK_IMPORTED_MODULE_4__ = __webpack_require__(/*! !../node_modules/style-loader/dist/runtime/insertStyleElement.js */ \"./node_modules/style-loader/dist/runtime/insertStyleElement.js\");\n/* harmony import */ var _node_modules_style_loader_dist_runtime_insertStyleElement_js__WEBPACK_IMPORTED_MODULE_4___default = /*#__PURE__*/__webpack_require__.n(_node_modules_style_loader_dist_runtime_insertStyleElement_js__WEBPACK_IMPORTED_MODULE_4__);\n/* harmony import */ var _node_modules_style_loader_dist_runtime_styleTagTransform_js__WEBPACK_IMPORTED_MODULE_5__ = __webpack_require__(/*! !../node_modules/style-loader/dist/runtime/styleTagTransform.js */ \"./node_modules/style-loader/dist/runtime/styleTagTransform.js\");\n/* harmony import */ var _node_modules_style_loader_dist_runtime_styleTagTransform_js__WEBPACK_IMPORTED_MODULE_5___default = /*#__PURE__*/__webpack_require__.n(_node_modules_style_loader_dist_runtime_styleTagTransform_js__WEBPACK_IMPORTED_MODULE_5__);\n/* harmony import */ var _node_modules_css_loader_dist_cjs_js_node_modules_postcss_loader_dist_cjs_js_style_css__WEBPACK_IMPORTED_MODULE_6__ = __webpack_require__(/*! !!../node_modules/css-loader/dist/cjs.js!../node_modules/postcss-loader/dist/cjs.js!./style.css */ \"./node_modules/css-loader/dist/cjs.js!./node_modules/postcss-loader/dist/cjs.js!./src/style.css\");\n\n      \n      \n      \n      \n      \n      \n      \n      \n      \n\nvar options = {};\n\noptions.styleTagTransform = (_node_modules_style_loader_dist_runtime_styleTagTransform_js__WEBPACK_IMPORTED_MODULE_5___default());\noptions.setAttributes = (_node_modules_style_loader_dist_runtime_setAttributesWithoutAttributes_js__WEBPACK_IMPORTED_MODULE_3___default());\noptions.insert = _node_modules_style_loader_dist_runtime_insertBySelector_js__WEBPACK_IMPORTED_MODULE_2___default().bind(null, \"head\");\noptions.domAPI = (_node_modules_style_loader_dist_runtime_styleDomAPI_js__WEBPACK_IMPORTED_MODULE_1___default());\noptions.insertStyleElement = (_node_modules_style_loader_dist_runtime_insertStyleElement_js__WEBPACK_IMPORTED_MODULE_4___default());\n\nvar update = _node_modules_style_loader_dist_runtime_injectStylesIntoStyleTag_js__WEBPACK_IMPORTED_MODULE_0___default()(_node_modules_css_loader_dist_cjs_js_node_modules_postcss_loader_dist_cjs_js_style_css__WEBPACK_IMPORTED_MODULE_6__[\"default\"], options);\n\n\n\n\n       /* harmony default export */ const __WEBPACK_DEFAULT_EXPORT__ = (_node_modules_css_loader_dist_cjs_js_node_modules_postcss_loader_dist_cjs_js_style_css__WEBPACK_IMPORTED_MODULE_6__[\"default\"] && _node_modules_css_loader_dist_cjs_js_node_modules_postcss_loader_dist_cjs_js_style_css__WEBPACK_IMPORTED_MODULE_6__[\"default\"].locals ? _node_modules_css_loader_dist_cjs_js_node_modules_postcss_loader_dist_cjs_js_style_css__WEBPACK_IMPORTED_MODULE_6__[\"default\"].locals : undefined);\n\n\n//# sourceURL=webpack://nova-protocol-web/./src/style.css?\n}");

/***/ },

/***/ "./node_modules/style-loader/dist/runtime/injectStylesIntoStyleTag.js"
/*!****************************************************************************!*\
  !*** ./node_modules/style-loader/dist/runtime/injectStylesIntoStyleTag.js ***!
  \****************************************************************************/
(module) {

eval("{\n\nvar stylesInDOM = [];\nfunction getIndexByIdentifier(identifier) {\n  var result = -1;\n  for (var i = 0; i < stylesInDOM.length; i++) {\n    if (stylesInDOM[i].identifier === identifier) {\n      result = i;\n      break;\n    }\n  }\n  return result;\n}\nfunction modulesToDom(list, options) {\n  var idCountMap = {};\n  var identifiers = [];\n  for (var i = 0; i < list.length; i++) {\n    var item = list[i];\n    var id = options.base ? item[0] + options.base : item[0];\n    var count = idCountMap[id] || 0;\n    var identifier = \"\".concat(id, \" \").concat(count);\n    idCountMap[id] = count + 1;\n    var indexByIdentifier = getIndexByIdentifier(identifier);\n    var obj = {\n      css: item[1],\n      media: item[2],\n      sourceMap: item[3],\n      supports: item[4],\n      layer: item[5]\n    };\n    if (indexByIdentifier !== -1) {\n      stylesInDOM[indexByIdentifier].references++;\n      stylesInDOM[indexByIdentifier].updater(obj);\n    } else {\n      var updater = addElementStyle(obj, options);\n      options.byIndex = i;\n      stylesInDOM.splice(i, 0, {\n        identifier: identifier,\n        updater: updater,\n        references: 1\n      });\n    }\n    identifiers.push(identifier);\n  }\n  return identifiers;\n}\nfunction addElementStyle(obj, options) {\n  var api = options.domAPI(options);\n  api.update(obj);\n  var updater = function updater(newObj) {\n    if (newObj) {\n      if (newObj.css === obj.css && newObj.media === obj.media && newObj.sourceMap === obj.sourceMap && newObj.supports === obj.supports && newObj.layer === obj.layer) {\n        return;\n      }\n      api.update(obj = newObj);\n    } else {\n      api.remove();\n    }\n  };\n  return updater;\n}\nmodule.exports = function (list, options) {\n  options = options || {};\n  list = list || [];\n  var lastIdentifiers = modulesToDom(list, options);\n  return function update(newList) {\n    newList = newList || [];\n    for (var i = 0; i < lastIdentifiers.length; i++) {\n      var identifier = lastIdentifiers[i];\n      var index = getIndexByIdentifier(identifier);\n      stylesInDOM[index].references--;\n    }\n    var newLastIdentifiers = modulesToDom(newList, options);\n    for (var _i = 0; _i < lastIdentifiers.length; _i++) {\n      var _identifier = lastIdentifiers[_i];\n      var _index = getIndexByIdentifier(_identifier);\n      if (stylesInDOM[_index].references === 0) {\n        stylesInDOM[_index].updater();\n        stylesInDOM.splice(_index, 1);\n      }\n    }\n    lastIdentifiers = newLastIdentifiers;\n  };\n};\n\n//# sourceURL=webpack://nova-protocol-web/./node_modules/style-loader/dist/runtime/injectStylesIntoStyleTag.js?\n}");

/***/ },

/***/ "./node_modules/style-loader/dist/runtime/insertBySelector.js"
/*!********************************************************************!*\
  !*** ./node_modules/style-loader/dist/runtime/insertBySelector.js ***!
  \********************************************************************/
(module) {

eval("{\n\nvar memo = {};\n\n/* istanbul ignore next  */\nfunction getTarget(target) {\n  if (typeof memo[target] === \"undefined\") {\n    var styleTarget = document.querySelector(target);\n\n    // Special case to return head of iframe instead of iframe itself\n    if (window.HTMLIFrameElement && styleTarget instanceof window.HTMLIFrameElement) {\n      try {\n        // This will throw an exception if access to iframe is blocked\n        // due to cross-origin restrictions\n        styleTarget = styleTarget.contentDocument.head;\n      } catch (e) {\n        // istanbul ignore next\n        styleTarget = null;\n      }\n    }\n    memo[target] = styleTarget;\n  }\n  return memo[target];\n}\n\n/* istanbul ignore next  */\nfunction insertBySelector(insert, style) {\n  var target = getTarget(insert);\n  if (!target) {\n    throw new Error(\"Couldn't find a style target. This probably means that the value for the 'insert' parameter is invalid.\");\n  }\n  target.appendChild(style);\n}\nmodule.exports = insertBySelector;\n\n//# sourceURL=webpack://nova-protocol-web/./node_modules/style-loader/dist/runtime/insertBySelector.js?\n}");

/***/ },

/***/ "./node_modules/style-loader/dist/runtime/insertStyleElement.js"
/*!**********************************************************************!*\
  !*** ./node_modules/style-loader/dist/runtime/insertStyleElement.js ***!
  \**********************************************************************/
(module) {

eval("{\n\n/* istanbul ignore next  */\nfunction insertStyleElement(options) {\n  var element = document.createElement(\"style\");\n  options.setAttributes(element, options.attributes);\n  options.insert(element, options.options);\n  return element;\n}\nmodule.exports = insertStyleElement;\n\n//# sourceURL=webpack://nova-protocol-web/./node_modules/style-loader/dist/runtime/insertStyleElement.js?\n}");

/***/ },

/***/ "./node_modules/style-loader/dist/runtime/setAttributesWithoutAttributes.js"
/*!**********************************************************************************!*\
  !*** ./node_modules/style-loader/dist/runtime/setAttributesWithoutAttributes.js ***!
  \**********************************************************************************/
(module, __unused_webpack_exports, __webpack_require__) {

eval("{\n\n/* istanbul ignore next  */\nfunction setAttributesWithoutAttributes(styleElement) {\n  var nonce =  true ? __webpack_require__.nc : 0;\n  if (nonce) {\n    styleElement.setAttribute(\"nonce\", nonce);\n  }\n}\nmodule.exports = setAttributesWithoutAttributes;\n\n//# sourceURL=webpack://nova-protocol-web/./node_modules/style-loader/dist/runtime/setAttributesWithoutAttributes.js?\n}");

/***/ },

/***/ "./node_modules/style-loader/dist/runtime/styleDomAPI.js"
/*!***************************************************************!*\
  !*** ./node_modules/style-loader/dist/runtime/styleDomAPI.js ***!
  \***************************************************************/
(module) {

eval("{\n\n/* istanbul ignore next  */\nfunction apply(styleElement, options, obj) {\n  var css = \"\";\n  if (obj.supports) {\n    css += \"@supports (\".concat(obj.supports, \") {\");\n  }\n  if (obj.media) {\n    css += \"@media \".concat(obj.media, \" {\");\n  }\n  var needLayer = typeof obj.layer !== \"undefined\";\n  if (needLayer) {\n    css += \"@layer\".concat(obj.layer.length > 0 ? \" \".concat(obj.layer) : \"\", \" {\");\n  }\n  css += obj.css;\n  if (needLayer) {\n    css += \"}\";\n  }\n  if (obj.media) {\n    css += \"}\";\n  }\n  if (obj.supports) {\n    css += \"}\";\n  }\n  var sourceMap = obj.sourceMap;\n  if (sourceMap && typeof btoa !== \"undefined\") {\n    css += \"\\n/*# sourceMappingURL=data:application/json;base64,\".concat(btoa(unescape(encodeURIComponent(JSON.stringify(sourceMap)))), \" */\");\n  }\n\n  // For old IE\n  /* istanbul ignore if  */\n  options.styleTagTransform(css, styleElement, options.options);\n}\nfunction removeStyleElement(styleElement) {\n  // istanbul ignore if\n  if (styleElement.parentNode === null) {\n    return false;\n  }\n  styleElement.parentNode.removeChild(styleElement);\n}\n\n/* istanbul ignore next  */\nfunction domAPI(options) {\n  if (typeof document === \"undefined\") {\n    return {\n      update: function update() {},\n      remove: function remove() {}\n    };\n  }\n  var styleElement = options.insertStyleElement(options);\n  return {\n    update: function update(obj) {\n      apply(styleElement, options, obj);\n    },\n    remove: function remove() {\n      removeStyleElement(styleElement);\n    }\n  };\n}\nmodule.exports = domAPI;\n\n//# sourceURL=webpack://nova-protocol-web/./node_modules/style-loader/dist/runtime/styleDomAPI.js?\n}");

/***/ },

/***/ "./node_modules/style-loader/dist/runtime/styleTagTransform.js"
/*!*********************************************************************!*\
  !*** ./node_modules/style-loader/dist/runtime/styleTagTransform.js ***!
  \*********************************************************************/
(module) {

eval("{\n\n/* istanbul ignore next  */\nfunction styleTagTransform(css, styleElement) {\n  if (styleElement.styleSheet) {\n    styleElement.styleSheet.cssText = css;\n  } else {\n    while (styleElement.firstChild) {\n      styleElement.removeChild(styleElement.firstChild);\n    }\n    styleElement.appendChild(document.createTextNode(css));\n  }\n}\nmodule.exports = styleTagTransform;\n\n//# sourceURL=webpack://nova-protocol-web/./node_modules/style-loader/dist/runtime/styleTagTransform.js?\n}");

/***/ },

/***/ "./src/index.ts"
/*!**********************!*\
  !*** ./src/index.ts ***!
  \**********************/
(__unused_webpack_module, __webpack_exports__, __webpack_require__) {

eval("{__webpack_require__.r(__webpack_exports__);\n/* harmony import */ var _style_css__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./style.css */ \"./src/style.css\");\n/* harmony import */ var _site__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./site */ \"./src/site.ts\");\n\n\n(0,_site__WEBPACK_IMPORTED_MODULE_1__.initSite)();\n\n\n//# sourceURL=webpack://nova-protocol-web/./src/index.ts?\n}");

/***/ },

/***/ "./src/site.ts"
/*!*********************!*\
  !*** ./src/site.ts ***!
  \*********************/
(__unused_webpack_module, __webpack_exports__, __webpack_require__) {

eval("{__webpack_require__.r(__webpack_exports__);\n/* harmony export */ __webpack_require__.d(__webpack_exports__, {\n/* harmony export */   initSite: () => (/* binding */ initSite)\n/* harmony export */ });\nfunction initSite() {\n    const strip = (p) => p.replace(/\\/+$/, \"\");\n    const pathOf = (a) => strip(new URL(a.href, window.location.origin).pathname);\n    const current = strip(window.location.pathname);\n    const brand = document.querySelector(\".site-header__brand\");\n    const root = brand ? pathOf(brand) : \"\";\n    const links = document.querySelectorAll(\".site-nav a\");\n    links.forEach((link) => {\n        if (link.classList.contains(\"is-cta\"))\n            return;\n        const target = pathOf(link);\n        const active = current === target ||\n            (target !== root && current.startsWith(target + \"/\"));\n        if (active) {\n            link.setAttribute(\"aria-current\", \"page\");\n            link.style.color = \"var(--text)\";\n        }\n    });\n}\n\n\n//# sourceURL=webpack://nova-protocol-web/./src/site.ts?\n}");

/***/ }

/******/ 	});
/************************************************************************/
/******/ 	// The module cache
/******/ 	const __webpack_module_cache__ = {};
/******/ 	
/******/ 	// The require function
/******/ 	function __webpack_require__(moduleId) {
/******/ 		// Check if module is in cache
/******/ 		const cachedModule = __webpack_module_cache__[moduleId];
/******/ 		if (cachedModule !== undefined) {
/******/ 			return cachedModule.exports;
/******/ 		}
/******/ 		// Create a new module (and put it into the cache)
/******/ 		const module = __webpack_module_cache__[moduleId] = {
/******/ 			id: moduleId,
/******/ 			// no module.loaded needed
/******/ 			exports: {}
/******/ 		};
/******/ 	
/******/ 		// Execute the module function
/******/ 		if (!(moduleId in __webpack_modules__)) {
/******/ 			delete __webpack_module_cache__[moduleId];
/******/ 			const e = new Error("Cannot find module '" + moduleId + "'");
/******/ 			e.code = 'MODULE_NOT_FOUND';
/******/ 			throw e;
/******/ 		}
/******/ 		__webpack_modules__[moduleId](module, module.exports, __webpack_require__);
/******/ 	
/******/ 		// Return the exports of the module
/******/ 		return module.exports;
/******/ 	}
/******/ 	
/************************************************************************/
/******/ 	/* webpack/runtime/compat get default export */
/******/ 	(() => {
/******/ 		// getDefaultExport function for compatibility with non-harmony modules
/******/ 		__webpack_require__.n = (module) => {
/******/ 			const getter = module && module.__esModule ?
/******/ 				() => (module['default']) :
/******/ 				() => (module);
/******/ 			__webpack_require__.d(getter, { a: getter });
/******/ 			return getter;
/******/ 		};
/******/ 	})();
/******/ 	
/******/ 	/* webpack/runtime/define property getters */
/******/ 	(() => {
/******/ 		// define getter/value functions for harmony exports
/******/ 		__webpack_require__.d = (exports, definition) => {
/******/ 			if(Array.isArray(definition)) {
/******/ 				var i = 0;
/******/ 				while(i < definition.length) {
/******/ 					var key = definition[i++];
/******/ 					var binding = definition[i++];
/******/ 					if(!__webpack_require__.o(exports, key)) {
/******/ 						if(binding === 0) {
/******/ 							Object.defineProperty(exports, key, { enumerable: true, value: definition[i++] });
/******/ 						} else {
/******/ 							Object.defineProperty(exports, key, { enumerable: true, get: binding });
/******/ 						}
/******/ 					} else if(binding === 0) { i++; }
/******/ 				}
/******/ 			} else {
/******/ 				for(var key in definition) {
/******/ 					if(__webpack_require__.o(definition, key) && !__webpack_require__.o(exports, key)) {
/******/ 						Object.defineProperty(exports, key, { enumerable: true, get: definition[key] });
/******/ 					}
/******/ 				}
/******/ 			}
/******/ 		};
/******/ 	})();
/******/ 	
/******/ 	/* webpack/runtime/hasOwnProperty shorthand */
/******/ 	(() => {
/******/ 		__webpack_require__.o = (obj, prop) => (Object.prototype.hasOwnProperty.call(obj, prop))
/******/ 	})();
/******/ 	
/******/ 	/* webpack/runtime/make namespace object */
/******/ 	(() => {
/******/ 		// define __esModule on exports
/******/ 		__webpack_require__.r = (exports) => {
/******/ 			if(Symbol.toStringTag) {
/******/ 				Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });
/******/ 			}
/******/ 			Object.defineProperty(exports, '__esModule', { value: true });
/******/ 		};
/******/ 	})();
/******/ 	
/******/ 	/* webpack/runtime/nonce */
/******/ 	(() => {
/******/ 		__webpack_require__.nc = undefined;
/******/ 	})();
/******/ 	
/************************************************************************/
/******/ 	
/******/ 	// startup
/******/ 	// Load entry module and return exports
/******/ 	// This entry module can't be inlined because the eval devtool is used.
/******/ 	let __webpack_exports__ = __webpack_require__("./src/index.ts");
/******/ 	
/******/ })()
;