//! Isolated 54x54px Cairo 1 Hz iPhone Duo-style 20s Circular Focus Countdown Widget
//! - Runs ONLY while viewing a card Front (built-in 20s, zero settings clutter)
//! - Redraws strictly its own 54x54px dirty rect (never triggers WebView reflow)
//! - Monotonic black->white clockwise sweep; at 0s renders clean EMPTY hollow ring (no dot)
use std::f64::consts::PI;
use std::ffi::CString;
use std::ptr;

use crate::ffi::cairo::*;
use crate::ffi::gtk::*;
use crate::logic::FocusTimerState;
use crate::ui::theme::{fill_white, TIMER_SIZE};

pub struct FocusTimerWidget {
    pub drawing_area: *mut GtkWidget,
    pub state: FocusTimerState,
    pub timeout_id: Option<guint>,
}

impl FocusTimerWidget {
    pub unsafe fn new(
        expose_cb: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventExpose, gpointer) -> gboolean,
    ) -> Self {
        let da = gtk_drawing_area_new();
        gtk_widget_set_size_request(da, TIMER_SIZE, TIMER_SIZE);
        let sig = CString::new("expose-event").unwrap();
        g_signal_connect_data(
            da as gpointer,
            sig.as_ptr(),
            Some(std::mem::transmute(expose_cb as usize)),
            ptr::null_mut(),
            None,
            0,
        );

        Self {
            drawing_area: da,
            state: FocusTimerState::Hidden,
            timeout_id: None,
        }
    }

    pub unsafe fn stop_source(&mut self) {
        if let Some(id) = self.timeout_id.take() {
            g_source_remove(id);
        }
    }

    pub unsafe fn start_front(
        &mut self,
        tick_cb: unsafe extern "C" fn(gpointer) -> gboolean,
    ) {
        self.stop_source();
        self.state = FocusTimerState::start_card_front();
        self.timeout_id = Some(g_timeout_add(1000, tick_cb, ptr::null_mut()));
        self.queue_isolated_redraw();
    }

    pub unsafe fn hide(&mut self) {
        self.stop_source();
        if self.state != FocusTimerState::Hidden {
            self.state = FocusTimerState::hide();
            self.queue_isolated_redraw();
        }
    }

    pub unsafe fn on_tick(&mut self) -> bool {
        let keep = self.state.tick();
        if !keep {
            self.timeout_id = None;
        }
        self.queue_isolated_redraw();
        keep
    }

    pub unsafe fn queue_isolated_redraw(&self) {
        gtk_widget_queue_draw_area(self.drawing_area, 0, 0, TIMER_SIZE, TIMER_SIZE);
    }

    pub unsafe fn draw(&self, cr: *mut cairo_t) {
        fill_white(cr);

        let frac = match self.state.remaining_fraction() {
            Some(f) => f,
            None => return, // Hidden: blank white
        };

        let cx = (TIMER_SIZE as f64) / 2.0;
        let cy = (TIMER_SIZE as f64) / 2.0;
        let r_outer = 22.0;
        let r_fill = 18.0;

        // Crisp outer monochrome ring
        cairo_set_source_rgb(cr, 0.0, 0.0, 0.0);
        cairo_set_line_width(cr, 2.2);
        cairo_new_path(cr);
        cairo_arc(cr, cx, cy, r_outer, 0.0, 2.0 * PI);
        cairo_stroke(cr);

        // At 0s (Expired): keep center completely EMPTY hollow ring (no dot!)
        if frac <= 0.0 {
            return;
        }

        // Remaining clockwise-shrinking filled monochrome pie sector (from 12 o'clock)
        let start_angle = -PI / 2.0 + (1.0 - frac) * 2.0 * PI;
        let end_angle = -PI / 2.0 + 2.0 * PI;
        cairo_new_path(cr);
        cairo_move_to(cr, cx, cy);
        cairo_arc(cr, cx, cy, r_fill, start_angle, end_angle);
        cairo_close_path(cr);
        cairo_fill(cr);
    }
}
