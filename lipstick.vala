// Minimal LIPC bindings for Kindle on-screen keyboard (Lipstick)
using GLib;

public errordomain LipstickError {
    ERROR
}

public class Lipstick : GLib.Object {
    private void* handle;

    [CCode (cname = "LipcOpenNoName")]
    private extern static void* lipc_open_no_name ();

    [CCode (cname = "LipcClose")]
    private extern static void lipc_close (void* handle);

    [CCode (cname = "LipcGetIntProperty")]
    private extern static int lipc_get_int_property (void* handle, string service, string property, out int value);

    [CCode (cname = "LipcSetIntProperty")]
    private extern static int lipc_set_int_property (void* handle, string service, string property, int value);

    [CCode (cname = "LipcGetStringProperty")]
    private extern static int lipc_get_string_property (void* handle, string service, string property, out string value);

    [CCode (cname = "LipcSetStringProperty")]
    private extern static int lipc_set_string_property (void* handle, string service, string property, string value);

    public Lipstick () throws LipstickError {
        handle = lipc_open_no_name ();
        if (handle == null) {
            throw new LipstickError.ERROR ("Failed to open LIPC handle");
        }
    }

    ~Lipstick () {
        if (handle != null) {
            lipc_close (handle);
            handle = null;
        }
    }

    public int get_int_property (string service, string property) throws LipstickError {
        if (handle == null) return 0;
        int val = 0;
        int ret = lipc_get_int_property (handle, service, property, out val);
        if (ret != 0) {
            throw new LipstickError.ERROR (@"Failed to get int property $property on $service (code $ret)");
        }
        return val;
    }

    public void set_int_property (string service, string property, int val) throws LipstickError {
        if (handle == null) return;
        int ret = lipc_set_int_property (handle, service, property, val);
        if (ret != 0) {
            throw new LipstickError.ERROR (@"Failed to set int property $property on $service (code $ret)");
        }
    }

    public string get_string_property (string service, string property) throws LipstickError {
        if (handle == null) return "";
        string val = "";
        int ret = lipc_get_string_property (handle, service, property, out val);
        if (ret != 0) {
            throw new LipstickError.ERROR (@"Failed to get string property $property on $service (code $ret)");
        }
        return val;
    }

    public void set_string_property (string service, string property, string val) throws LipstickError {
        if (handle == null) return;
        int ret = lipc_set_string_property (handle, service, property, val);
        if (ret != 0) {
            throw new LipstickError.ERROR (@"Failed to set string property $property on $service (code $ret)");
        }
    }
}
