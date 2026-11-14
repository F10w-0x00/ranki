using Gtk;
using Gdk;
using GLib;

public class InkButton : Gtk.Button {
    public int ink_border { get; set; default = 1; }

    public InkButton () {
    }

    public override bool expose_event (Gdk.EventExpose event) {
        bool res = base.expose_event (event);
        if (ink_border > 0 && window != null) {
            Cairo.Context cr = Gdk.cairo_create (window);
            cr.set_line_width (ink_border);
            cr.set_source_rgb (0, 0, 0);
            cr.rectangle (allocation.x + 0.5, allocation.y + 0.5, allocation.width - 1.0, allocation.height - 1.0);
            cr.stroke ();
        }
        return res;
    }
}

public class InkEntry : Gtk.Entry {
    public InkEntry () {
    }
}

public class InkHScale : Gtk.HScale {
    public InkHScale () {
    }
}

public class InkRadioButton : Gtk.RadioButton {
    public InkRadioButton () {
    }
}

public class InkToggleButton : Gtk.ToggleButton {
    public InkToggleButton () {
    }
}

public class InkScrolledWindow : Gtk.ScrolledWindow {
    public bool is_swipe { get; set; default = false; }
    public bool expand { get; set; default = false; }

    public InkScrolledWindow () {
    }
}

public class InkTreeView : Gtk.TreeView {
    public bool is_swipe { get; set; default = false; }
    public signal void path_selected (Gtk.TreePath path);

    public InkTreeView () {
        this.cursor_changed.connect (() => {
            Gtk.TreePath path;
            Gtk.TreeViewColumn col;
            get_cursor (out path, out col);
            if (path != null) {
                path_selected (path);
            }
        });
    }

    public override bool button_release_event (Gdk.EventButton event) {
        bool res = base.button_release_event (event);
        Gtk.TreePath path;
        if (get_path_at_pos ((int) event.x, (int) event.y, out path, null, null, null)) {
            if (path != null) {
                path_selected (path);
            }
        }
        return res;
    }
}

public class InkRowToggleButton : Gtk.HBox {
    private Gtk.Label title_label;
    private Gtk.Label desc_label;
    private Gtk.CheckButton toggle_btn;

    private string _label = "";
    public string label {
        get { return _label; }
        set {
            _label = value;
            update_labels ();
        }
    }

    private string _description = "";
    public string description {
        get { return _description; }
        set {
            _description = value;
            update_labels ();
        }
    }

    public bool active {
        get { return toggle_btn.active; }
        set { toggle_btn.active = value; }
    }

    public signal void toggled ();

    public InkRowToggleButton () {
        this.spacing = 10;
        this.homogeneous = false;

        var vbox = new Gtk.VBox (false, 2);
        title_label = new Gtk.Label ("");
        title_label.xalign = 0.0f;
        title_label.use_markup = true;

        desc_label = new Gtk.Label ("");
        desc_label.xalign = 0.0f;
        desc_label.use_markup = true;

        vbox.pack_start (title_label, false, false, 0);
        vbox.pack_start (desc_label, false, false, 0);

        toggle_btn = new Gtk.CheckButton ();
        toggle_btn.toggled.connect (() => {
            toggled ();
            notify_property ("active");
        });

        pack_start (vbox, true, true, 0);
        pack_end (toggle_btn, false, false, 0);

        show_all ();
    }

    private void update_labels () {
        title_label.set_markup ("<b>" + GLib.Markup.escape_text (_label) + "</b>");
        desc_label.set_text (_description);
    }
}

public class InkRowHScale : Gtk.VBox {
    private Gtk.Label title_label;
    private Gtk.Label desc_label;
    private Gtk.HScale scale;
    private Gtk.Adjustment adjustment;

    private string _label = "";
    public string label {
        get { return _label; }
        set {
            _label = value;
            update_labels ();
        }
    }

    private string _description = "";
    public string description {
        get { return _description; }
        set {
            _description = value;
            update_labels ();
        }
    }

    public double value {
        get { return scale.get_value (); }
        set { scale.set_value (value); }
    }

    public int digits {
        get { return scale.get_digits (); }
        set { scale.set_digits (value); }
    }

    public double lower {
        get { return adjustment.lower; }
        set {
            adjustment.lower = value;
            adjustment.changed ();
        }
    }

    public double upper {
        get { return adjustment.upper; }
        set {
            adjustment.upper = value;
            adjustment.changed ();
        }
    }

    public signal void value_changed ();

    public InkRowHScale () {
        this.spacing = 4;
        this.homogeneous = false;

        title_label = new Gtk.Label ("");
        title_label.xalign = 0.0f;
        title_label.use_markup = true;

        desc_label = new Gtk.Label ("");
        desc_label.xalign = 0.0f;
        desc_label.use_markup = true;

        adjustment = new Gtk.Adjustment (1.0, 1.0, 5.0, 0.1, 0.5, 0.0);
        scale = new Gtk.HScale (adjustment);
        scale.set_digits (1);
        scale.value_changed.connect (() => {
            value_changed ();
            notify_property ("value");
        });

        pack_start (title_label, false, false, 0);
        pack_start (desc_label, false, false, 0);
        pack_start (scale, false, false, 0);

        show_all ();
    }

    private void update_labels () {
        title_label.set_markup ("<b>" + GLib.Markup.escape_text (_label) + "</b>");
        desc_label.set_text (_description);
    }
}
