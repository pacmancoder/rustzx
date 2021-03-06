//! Contains ZXSpectrum border implementation
use super::colors::*;
use utils::Clocks;
use zx::constants::*;
use zx::machine::*;

/// Internal struct, which contains information about beam position and color
#[derive(Clone, Copy)]
struct BeamInfo {
    line: usize,
    pixel: usize,
    color: ZXColor,
}
impl BeamInfo {
    /// constructs self with given color at first pixel pos
    fn first_pixel(color: ZXColor) -> BeamInfo {
        BeamInfo::new(0, 0, color)
    }
    /// constructs self at given pos with given color
    fn new(line: usize, pixel: usize, color: ZXColor) -> BeamInfo {
        BeamInfo {
            line: line,
            pixel: pixel,
            color: color,
        }
    }
    /// checks if beam is on first pixel
    fn is_first_pixel(&self) -> bool {
        (self.line == 0) && (self.pixel == 0)
    }
    /// resets position
    fn reset(&mut self) {
        self.line = 0;
        self.pixel = 0;
    }
}

/// ZX Spectrum Border Device
pub struct ZXBorder {
    machine: ZXMachine,
    palette: ZXPalette,
    buffer: Box<[u8; PIXEL_COUNT * BYTES_PER_PIXEL]>,
    beam_last: BeamInfo,
    border_changed: bool,
    beam_block: bool,
}
impl ZXBorder {
    /// Returns new instance of border device
    pub fn new(machine: ZXMachine, palette: ZXPalette) -> ZXBorder {
        ZXBorder {
            machine: machine,
            palette: palette,
            buffer: Box::new([0; PIXEL_COUNT * BYTES_PER_PIXEL]),
            beam_last: BeamInfo::first_pixel(ZXColor::White),
            border_changed: true,
            beam_block: false,
        }
    }

    /// ULA draws 2 pixels per TState.
    /// This function helps to determine pixel, which will be rendered at specific time
    /// and bool value, which signals end of frame
    fn next_border_pixel(&self, clocks: Clocks) -> (usize, usize, bool) {
        let specs = self.machine.specs();
        // begining of the first line (first pixel timing minus border lines
        // minus left border columns)
        let clocks_origin = specs.clocks_first_pixel
            - 8 * BORDER_ROWS * specs.clocks_line as usize
            - BORDER_COLS * CLOCKS_PER_COL
            + specs.clocks_ula_beam_shift;
        // return first pixel pos
        if clocks.count() < clocks_origin {
            return (0, 0, false);
        }
        // get clocks relative to first pixel
        let clocks = clocks.count() - clocks_origin;
        let mut line = clocks / specs.clocks_line as usize;
        // so, next pixel will be current + 2
        let mut pixel = ((clocks % specs.clocks_line as usize) + 1) * PIXELS_PER_CLOCK as usize;
        // if beam out of screen on horizontal pos.
        // pixel - 2 bacause we added 2 on prev line
        if pixel - PIXELS_PER_CLOCK >= SCREEN_WIDTH {
            // first pixel of next line
            pixel = 0;
            line += 1;
        }
        // if beam out of screen on vertical pos.
        if line >= SCREEN_HEIGHT {
            return (0, 0, true);
        } else {
            return (line, pixel, false);
        }
    }

    /// fills pixels from last pos to passed by arguments with
    fn fill_to(&mut self, line: usize, pixel: usize) {
        let last = self.beam_last;
        let color_array = self.palette.get_rgba(last.color, ZXBrightness::Normal);
        // fill pixels
        for p in (last.line * SCREEN_WIDTH + last.pixel)..(line * SCREEN_WIDTH + pixel) {
            // all 4 bytes of color
            for b in 0..BYTES_PER_PIXEL {
                self.buffer[p * BYTES_PER_PIXEL + b] = color_array[b];
            }
        }
    }

    /// starts new frame
    pub fn new_frame(&mut self) {
        // if border was not changed during prev frame then force change color of whole border
        if !self.border_changed {
            self.beam_last.reset();
        }
        // fill to end of screen if not already filled
        if !self.beam_block {
            self.fill_to(SCREEN_HEIGHT - 1, SCREEN_WIDTH);
        }
        // move beam to begin and reset flags
        self.beam_last.reset();
        self.border_changed = false;
        self.beam_block = false;
    }

    /// changes color of border
    pub fn set_border(&mut self, clocks: Clocks, color: ZXColor) {
        // border updated during frame
        self.border_changed = true;
        let (line, pixel, frame_end) = self.next_border_pixel(clocks);
        if !self.beam_block {
            // if not first pixel then update
            if frame_end {
                self.fill_to(SCREEN_HEIGHT - 1, SCREEN_WIDTH);
                self.beam_block = true;
            }
            self.fill_to(line, pixel);
        }
        self.beam_last = BeamInfo::new(line, pixel, color);
    }

    /// Returns reference to texture
    pub fn texture(&self) -> &[u8] {
        &(*self.buffer)
    }
}
