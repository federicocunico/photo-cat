# Offline world geometry

- `countries-50m.json`: Natural Earth 1:50m admin-0 countries as quantized TopoJSON
  (`objects.countries`, `objects.land`).
- Source: npm package `world-atlas@2.0.2` (`countries-50m.json`), https://github.com/topojson/world-atlas
- Licences: Natural Earth data is public domain (https://www.naturalearthdata.com/about/terms-of-use/);
  the `world-atlas` packaging is ISC (Copyright Mike Bostock).
- Shipped with the app and loaded locally; no network access at runtime.
