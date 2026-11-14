# Checking the map's script

`map.js` drives a generated `bench-hashes.map.html` in Chromium: every chart
opens in place and closes, with its headers lit and the values under the
pointer; a header greys its charts; every call name links to its
documentation; a link to one chart opens it.

    npm install playwright
    node tools/graph-check/map.js benchmark-results/<machine>/bench-hashes.map.html /usr/bin/chromium

`checkPhoneTargets(phone)`, exported by `map.js`, takes a freshly loaded,
touch-enabled page. It checks centre and call-name taps, the plot's tooltip,
the close button and the enlarged chart's pointer-active documentation link,
leaving its first chart open. The full check then exercises the legend. Historical
maps share the target checks without interpreting their older data or legend forms.
