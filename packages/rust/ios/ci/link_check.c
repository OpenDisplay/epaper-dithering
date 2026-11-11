/* Link check for the built XCFramework: references every exported symbol, so linking this
 * against the iOS device slice fails if an export is missing or the archive is unreadable
 * by the installed Xcode toolchain. Run by .github/workflows/ios-test.yml. */
#include "epaper_dithering.h"

static const char PALETTE[] =
    "{\"format\":\"epaper-palette\",\"version\":1,\"entries\":["
    "{\"appearance\":[0,0,0]},{\"appearance\":[100,0,0]}]}";

int main(void) {
    ed_palette *palette = NULL;
    int32_t status = ed_palette_from_json(PALETTE, sizeof(PALETTE) - 1, &palette);
    ed_dither_options opts = ed_dither_options_default();
    opts.dbs_max_passes = ed_dither_options_default_dbs_passes();
    const uint8_t px[3] = {0, 0, 0};
    uint8_t out[1];
    uint8_t rgb[6];
    status += ed_palette_preview_rgb(palette, rgb, 3 * ed_palette_len(palette));
    status += ed_dither(px, 3, 1, palette, &opts, out, 1);
    ed_palette_free(palette);
    return (int)(status + (int32_t)ed_abi_version());
}
