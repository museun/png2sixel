#![allow(dead_code)]
use image::{GenericImage, Pixel};
use std::io::prelude::*;
use std::os::raw::{c_int, c_uchar, c_void};

type Alloc = *mut c_void;
type Dither = *mut c_void;
type Output = *mut c_void;
type Status = c_int;

type WriteFn = extern "C" fn(data: *mut c_uchar, len: c_int, userdata: *mut c_void) -> Status;

#[link(name = "sixel")]
extern "C" {
    fn sixel_output_new(
        output: *mut Output,
        write_fn: WriteFn,
        userdata: *mut c_void,
        alloc: Alloc,
    ) -> Status;

    fn sixel_dither_get(kind: c_int) -> Dither;

    fn sixel_encode(
        data: *mut c_uchar,
        width: c_int,
        height: c_int,
        _: c_int,
        dither: Dither,
        output: Output,
    ) -> Status;
}

#[derive(Copy, Clone, Debug)]
pub struct Sixel;
impl Sixel {
    fn write<W: Write, I: GenericImage<Pixel = P>, P: Pixel<Subpixel = u8>>(
        write: &mut W,
        image: &I,
    ) -> std::io::Result<()> {
        extern "C" fn write_fn(data: *mut c_uchar, len: c_int, userdata: *mut c_void) -> Status {
            unsafe {
                let output: &mut Vec<u8> = &mut *(userdata as *mut _);
                output
                    .write_all(std::slice::from_raw_parts(data, len as usize))
                    .unwrap();
                0
            }
        }

        let mut data = Vec::with_capacity(image.width() as usize * image.height() as usize * 3);
        for y in 0..image.height() {
            for x in 0..image.width() {
                let pixel = image.get_pixel(x, y).to_rgb();
                data.extend_from_slice(&pixel.data);
            }
        }

        let data: *mut c_uchar = data.as_mut_ptr() as *mut c_uchar;
        let mut output: Vec<u8> = Vec::new();
        let mut sixel = std::ptr::null_mut();

        if unsafe {
            sixel_output_new(
                &mut sixel,
                write_fn,
                &mut output as *mut _ as *mut _,
                std::ptr::null_mut(),
            )
        } != 0
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "sixel_output_new error",
            ));
        }

        let dither = unsafe { sixel_dither_get(3) };
        if unsafe {
            sixel_encode(
                data,
                image.width() as i32,
                image.height() as i32,
                0,
                dither,
                sixel,
            )
        } == 0
        {
            write.write_all(&output)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "sixel_encode error",
            ))
        }
    }
}

fn main() {
    let input = std::env::args().nth(1).unwrap();
    let image = image::open(input).unwrap();
    let mut out = vec![];
    Sixel::write(&mut out, &image).unwrap();
    eprintln!("{}", std::str::from_utf8(&out).unwrap());
}

