/* Link check for the built XCFramework: references every exported symbol, so linking this
 * against the iOS device slice fails if an export is missing or the archive is unreadable
 * by the installed Xcode toolchain. Run by .github/workflows/ios-test.yml. */
#include "epaper_dithering.h"

int main(void) {
    ed_dither_options opts = ed_dither_options_default();
    opts.dbs_max_passes = ed_dither_options_default_dbs_passes();
    const uint8_t px[3] = {0, 0, 0};
    const uint8_t pal[6] = {0, 0, 0, 255, 255, 255};
    uint8_t out[1];
    int32_t status = ed_dither(px, 3, 1, pal, 6, 1, NULL, 0, 0, &opts, out, 1);
    return (int)(status + (int32_t)ed_abi_version());
}
