use std::mem;
use std::slice;
use std::os::raw::c_void;


#[no_mangle]
pub extern "C" fn malloc(size: usize) -> *mut c_void {
    let mut buf = Vec::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    mem::forget(buf);
    return ptr as *mut c_void;
}

#[no_mangle]
pub extern "C" fn dealloc(ptr: *mut c_void, size: usize) {
    unsafe {
        let _buf = Vec::from_raw_parts(ptr, 0, size);
    }
}

fn greyscale(r: u8, g: u8, b: u8) -> u8 {
    ((r as usize + g as usize + b as usize) / 3) as u8
}

fn quantize(x: u8, levels: u8) -> u8 {
    // This uses f64.round() which requires an import of "roundf" when instantiating the WASM
    // module
    //((levels as f64 * (x as f64 / 256f64)).round() * (255f64 / levels as f64)) as u8

    // This method uses casts to perform f64.round() meaning that it requires no support from JS
    // when instantiating the WASM module.
    (
        (
            (
                // Instead of f64.round(), add 0.5 to result and convert to u32
                ((levels as f64 * (x as f64 / 256f64)) + 0.5f64) as u32

            // Convert back to f64: value is now rounded to nearest integer as with f64.round()
            ) as f64

        ) * (255f64 / levels as f64)
    ) as u8
}

fn err_dist(input: u8, error: f64, amount: f64) -> u8 {
    let v = (input as f64 + error * amount) as i32;

    // Clamp v to 0..256

    // If v is not in 0..256, invert v, shift to byte range and mask
    if v & !0xFF != 0 { (((!v) >> 31) & 0xFF) as u8 } else { v as u8 }

    // Logical alternative
    //if v < 0 { 0 } else if v > 255 { 255 } else { v as u8 }
}

fn process_buffer_dither(sl: &mut [u8], width: usize, height: usize, levels: u8, colour: bool) {
    let f1 = 7f64 / 16f64;
    let f2 = 3f64 / 16f64;
    let f3 = 5f64 / 16f64;
    let f4 = 1f64 / 16f64;

    let mut buf_off_x: usize; let mut buf_off_y: usize;
    let mut r: u8; let mut g: u8; let mut b: u8;
    let mut qr: u8; let mut qg: u8; let mut qb: u8;
    let mut err_r: f64; let mut err_g: f64; let mut err_b: f64;
    let mut err: f64;
    let mut v: u8; let mut v1: u8; let mut v2: u8; let mut v3: u8; let mut v4: u8;
    let mut qv: u8; let mut qv1: u8; let mut qv2: u8; let mut qv3: u8; let mut qv4: u8;

    if colour {
        for y in 0..height - 1 {
            buf_off_y = y * width * 4;
            for x in 1..width - 1 {
                buf_off_x = buf_off_y + (x * 4);

                r = sl[buf_off_x + 0];
                g = sl[buf_off_x + 1];
                b = sl[buf_off_x + 2];

                qr = quantize(r, levels);
                qg = quantize(g, levels);
                qb = quantize(b, levels);

                err_r = r as f64 - qr as f64;
                err_g = g as f64 - qg as f64;
                err_b = b as f64 - qb as f64;

                // x,y
                sl[buf_off_x + 0] = qr;
                sl[buf_off_x + 1] = qg;
                sl[buf_off_x + 2] = qb;

                if err_r as i64 != 0 {
                    sl[buf_off_x + 4 + 0] = err_dist(sl[buf_off_x + 4 + 0], err_r, f1);
                    sl[buf_off_x + (width * 4) - 4 + 0] = err_dist(sl[buf_off_x + (width * 4) - 4 + 0], err_r, f2);
                    sl[buf_off_x + (width * 4) + 0] = err_dist(sl[buf_off_x + (width * 4) + 0], err_r, f3);
                    sl[buf_off_x + (width * 4) + 4 + 0] = err_dist(sl[buf_off_x + (width * 4) + 4 + 0], err_r, f4);
                }

                if err_g as i64 != 0 {
                    sl[buf_off_x + 4 + 1] = err_dist(sl[buf_off_x + 4 + 1], err_g, f1);
                    sl[buf_off_x + (width * 4) - 4 + 1] = err_dist(sl[buf_off_x + (width * 4) - 4 + 1], err_g, f2);
                    sl[buf_off_x + (width * 4) + 1] = err_dist(sl[buf_off_x + (width * 4) + 1], err_g, f3);
                    sl[buf_off_x + (width * 4) + 4 + 1] = err_dist(sl[buf_off_x + (width * 4) + 4 + 1], err_g, f4);
                }

                if err_b as i64 != 0 {
                    sl[buf_off_x + 4 + 2] = err_dist(sl[buf_off_x + 4 + 2], err_b, f1);
                    sl[buf_off_x + (width * 4) - 4 + 2] = err_dist(sl[buf_off_x + (width * 4) - 4 + 2], err_b, f2);
                    sl[buf_off_x + (width * 4) + 2] = err_dist(sl[buf_off_x + (width * 4) + 2], err_b, f3);
                    sl[buf_off_x + (width * 4) + 4 + 2] = err_dist(sl[buf_off_x + (width * 4) + 4 + 2], err_b, f4);
                }
            }
        }
    } else {
        for y in 0..height - 1 {
            buf_off_y = y * width * 4;
            for x in 1..width - 1 {
                buf_off_x = buf_off_y + (x * 4);

                v = greyscale(sl[buf_off_x + 0], sl[buf_off_x + 1], sl[buf_off_x + 2]);
                qv = quantize(v, levels);
                err = v as f64 - qv as f64;

                // x,y
                sl[buf_off_x + 0] = qv;
                sl[buf_off_x + 1] = qv;
                sl[buf_off_x + 2] = qv;

                if err as i64 != 0 {
                    // x+1,y
                    v1 = greyscale(sl[buf_off_x + 4 + 0], sl[buf_off_x + 4 + 1], sl[buf_off_x + 4 + 2]);
                    qv1 = err_dist(v1, err, f1);
                    sl[buf_off_x + 4 + 0] = qv1;
                    sl[buf_off_x + 4 + 1] = qv1;
                    sl[buf_off_x + 4 + 2] = qv1;

                    // x-1,y+1
                    v2 = greyscale(sl[buf_off_x + (width * 4) - 4 + 0], sl[buf_off_x + (width * 4) - 4 + 1], sl[buf_off_x + (width * 4) - 4 + 2]);
                    qv2 = err_dist(v2, err, f2);
                    sl[buf_off_x + (width * 4) - 4 + 0] = qv2;
                    sl[buf_off_x + (width * 4) - 4 + 1] = qv2;
                    sl[buf_off_x + (width * 4) - 4 + 2] = qv2;

                    // x,y+1
                    v3 = greyscale(sl[buf_off_x + (width * 4) + 0], sl[buf_off_x + (width * 4) + 1], sl[buf_off_x + (width * 4) + 2]);
                    qv3 = err_dist(v3, err, f3);
                    sl[buf_off_x + (width * 4) + 0] = qv3;
                    sl[buf_off_x + (width * 4) + 1] = qv3;
                    sl[buf_off_x + (width * 4) + 2] = qv3;

                    // x+1,y+1
                    v4 = greyscale(sl[buf_off_x + (width * 4) + 4 + 0], sl[buf_off_x + (width * 4) + 4 + 1], sl[buf_off_x + (width * 4) + 4 + 2]);
                    qv4 = err_dist(v4, err, f4);
                    sl[buf_off_x + (width * 4) + 4 + 0] = qv4;
                    sl[buf_off_x + (width * 4) + 4 + 1] = qv4;
                    sl[buf_off_x + (width * 4) + 4 + 2] = qv4;
                }
            }
        }
    }
}

fn process_buffer_nodither(sl: &mut [u8], width: usize, height: usize, levels: u8, colour: bool) {
    let mut buf_off_x: usize; let mut buf_off_y: usize;
    let mut r: u8; let mut g: u8; let mut b: u8;
    let mut qr: u8; let mut qg: u8; let mut qb: u8;
    let mut v: u8;
    let mut qv: u8;

    if colour {
        for y in 0..height {
            buf_off_y = y * width * 4;
            for x in 0..width {
                buf_off_x = buf_off_y + (x * 4);

                r = sl[buf_off_x + 0];
                g = sl[buf_off_x + 1];
                b = sl[buf_off_x + 2];

                qr = quantize(r, levels);
                qg = quantize(g, levels);
                qb = quantize(b, levels);

                sl[buf_off_x + 0] = qr;
                sl[buf_off_x + 1] = qg;
                sl[buf_off_x + 2] = qb;
            }
        }
    } else {
        for y in 0..height {
            buf_off_y = y * width * 4;
            for x in 0..width {
                buf_off_x = buf_off_y + (x * 4);

                v = greyscale(sl[buf_off_x + 0], sl[buf_off_x + 1], sl[buf_off_x + 2]);
                qv = quantize(v, levels);

                sl[buf_off_x + 0] = qv;
                sl[buf_off_x + 1] = qv;
                sl[buf_off_x + 2] = qv;
            }
        }
    }
}


#[no_mangle]
pub extern "C" fn process_buffer(num_runs: usize, ptr: *mut u8, width: usize, height: usize, levels: u8, colour: bool, dither: bool) {
    if width == 0 || height == 0 {
        return;
    }

    let pixel_size = width * height;
    let byte_size  = pixel_size * 4;
    let sl = unsafe { slice::from_raw_parts_mut(ptr, byte_size) };

    for _ in 0..num_runs {
        if dither {
            process_buffer_dither(sl, width, height, levels, colour);
        }
        else {
            process_buffer_nodither(sl, width, height, levels, colour);
        }
    }
}


#[cfg(test)]
mod tests {
}
