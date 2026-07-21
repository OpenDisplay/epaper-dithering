def dither_image(
    pixels: bytes,
    width: int,
    height: int,
    *,
    scheme_id: int | None = ...,
    palette_bytes: bytes | None = ...,
    accent_idx: int = ...,
    mode_id: int = ...,
    serpentine: bool = ...,
    exposure: float = ...,
    saturation: float = ...,
    shadows: float = ...,
    highlights: float = ...,
    tone: float | None = ...,
    gamut: float | None = ...,
) -> bytes: ...
def composite_rgba(rgba: bytes) -> bytes: ...
def measured_palettes() -> list[tuple[str, list[int], list[str], int, int]]: ...
def tone_compress(pixels: list[float], palette_bytes: bytes, strength: float | None = ...) -> list[float]: ...
def gamut_compress(pixels: list[float], palette_bytes: bytes, strength: float | None = ...) -> list[float]: ...
def rgb_to_oklab_buffer(pixels: list[float]) -> list[float]: ...
