# AndroidSVG

Android uses the unmodified `com.caverock:androidsvg-aar:1.4` Maven Central AAR
(Apache-2.0); no native code or transitive dependencies. Version 1.4 is the release
identified by the [upstream download documentation](https://bigbadaboom.github.io/androidsvg/download.html).
The published 1.4 source archive was reviewed for `SVG.getFromInputStream`,
`setInternalEntitiesEnabled`, dimension/viewBox access, `renderToCanvas`, external
resolvers and embedded-raster decoding before adoption.

The app ships [attribution](../../android/app/src/main/assets/androidsvg/ATTRIBUTION.txt)
and the complete [license](../../android/app/src/main/assets/androidsvg/LICENSE-2.0.txt),
accessible offline from Settings. Sources and rendering API documentation:

- https://github.com/BigBadaboom/androidsvg
- https://bigbadaboom.github.io/androidsvg/doc/com/caverock/androidsvg/SVG.html
- https://repo.maven.apache.org/maven2/com/caverock/androidsvg-aar/1.4/androidsvg-aar-1.4-sources.jar

Reviewed Maven Central downloads (SHA-256):

- AAR: `02a5b08a2b35d2d58eb2eaca9d84ac00fb341da725fdbd653ea3ed130437e95a`
- Sources JAR: `b11595da341bb23c04a5fed0157babd630a44618f8e11c4f2b400167b0e0ce56`

Yomoku preflights SVG XML and local reference expansion, disables entity expansion,
installs no resource resolver and renders only onto a page-budgeted bitmap. SVGs
with nested image resources, DTDs, patterns/masks/markers, active/unsupported
elements or resource-valued
stylesheet rules use the existing alt-text/Retry fallback. Paths, shapes, text,
transforms, local uses, gradients and color/class styles remain supported within
the documented bounds. This introduces no browser/WebView or network loading.
