# Checking the map's script

`map.js` drives a generated `bench-hashes.map.html` in Chromium: every chart
opens in place and closes, with its headers lit and the values under the
pointer; a header greys its charts; every call name links to its
documentation; a link to one chart opens it.

    npm install playwright
    node tools/graph-check/map.js benchmark-results/<machine>/bench-hashes.map.html /usr/bin/chromium

`checkPhoneTargets(phone)`, exported by `map.js`, takes a freshly loaded,
touch-enabled page from the current renderer. It checks centre and call-name taps, the plot's tooltip,
the close button and the enlarged chart's pointer-active documentation link,
leaving its first chart open. The full check then exercises the legend. Historical
renderers use their version-matched checker; their interaction and measurement
contracts stay intact.

`checkZoomLayout(page)` expects an open current-format chart on a page with a
configured viewport. It checks heading separation, complete SVG labels, panel
bounds, the documentation link's hit target and the close button after scrolling,
then returns the panel to its top. The full checker calls it for every expanded
chart. Historical target checks retain their narrower interaction contract.
