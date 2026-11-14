using GLib;
using Archive;
using Soup;

namespace CrazySpices {

    [ CCode (cname = "realpath") ]
    public static extern string? realpath(string path, char *resolved_path = null);

    public delegate T WorkFunc<T> () throws Error;

    public static async T? run_thread<T> ( WorkFunc<T> callback = null ) throws Error {

        T? thread_output = null;
        Error? thread_error = null;

        ThreadFunc<void*> thread_fn = () => {
            try {
                thread_output = callback();
            } catch (Error e) {
                thread_error = e;
            }
            Idle.add (run_thread.callback);
            return null;
        };

        Thread<void*>.create (thread_fn, false);

        yield;

        if (thread_error != null) {
            throw thread_error;
        }

        return thread_output;
    }

    public delegate void DebounceFunc<T> (T val);

    public class Debouncer<T> : GLib.Object {
        private uint timeout_id = 0;
        private int delay_ms;
        private DebounceFunc<T> callback;
        private T last_val;

        public Debouncer (int delay_ms, owned DebounceFunc<T> cb) {
            this.delay_ms = delay_ms;
            this.callback = (owned) cb;
        }

        public void trigger (T val) {
            cancel ();
            this.last_val = val;
            timeout_id = GLib.Timeout.add (delay_ms, () => {
                timeout_id = 0;
                callback (last_val);
                return GLib.Source.REMOVE;
            });
        }

        public void cancel () {
            if (timeout_id != 0) {
                GLib.Source.remove (timeout_id);
                timeout_id = 0;
            }
        }

        ~Debouncer () {
            cancel ();
        }
    }

    public class NoodleSoupServer : GLib.Object {
        private Soup.Server server;
        public uint port { get; private set; default = 0; }
        private string root_dir;

        public NoodleSoupServer (string root_dir) {
            this.root_dir = root_dir;
            server = new Soup.Server (Soup.SERVER_PORT, 0, Soup.SERVER_INTERFACE, "127.0.0.1");
            server.add_handler ("/", on_request);
            try {
                server.run_async ();
                port = server.port;
                debug ("Listening on http://127.0.0.1:%u/ (serving %s)", port, root_dir);
            } catch (Error e) {
                warning ("Error starting NoodleSoupServer: %s", e.message);
            }
        }

        private void on_request (Soup.Server srv, Soup.Message msg, string path, GLib.HashTable? query, Soup.ClientContext client) {
            var file_path = Path.build_filename (root_dir, path.has_prefix ("/") ? path.substring (1) : path);
            var file = File.new_for_path (file_path);
            if (!file.query_exists ()) {
                msg.set_status (Soup.KnownStatusCode.NOT_FOUND);
                return;
            }

            try {
                uint8[] contents;
                string etag;
                file.load_contents (null, out contents, out etag);
                var mime = mime_for (file_path);
                msg.response_headers.set_content_type (mime, null);
                msg.response_headers.append ("Access-Control-Allow-Origin", "*");
                msg.response_headers.append ("Cache-Control", "public, max-age=3600");
                msg.set_response (mime, Soup.MemoryUse.COPY, contents);
                msg.set_status (Soup.KnownStatusCode.OK);
            } catch (Error e) {
                debug ("Error serving %s: %s", file_path, e.message);
                msg.set_status (Soup.KnownStatusCode.INTERNAL_SERVER_ERROR);
            }
        }

        public string mime_for (string path) {
            int idx = path.last_index_of_char ('.');
            string ext = (idx != -1) ? path.substring (idx + 1).down () : "";
            switch (ext) {
                case "html": return "text/html; charset=utf-8";
                case "css": return "text/css; charset=utf-8";
                case "js": return "application/javascript; charset=utf-8";
                case "woff": return "font/woff";
                case "woff2": return "font/woff2";
                case "ttf": return "font/ttf";
                case "otf": return "font/otf";
                case "svg": return "image/svg+xml";
                case "png": return "image/png";
                case "jpg":
                case "jpeg": return "image/jpeg";
                default: return "application/octet-stream";
            }
        }
    }

    public class Downloader : GLib.Object {
        public delegate void ProgressCallback (double now, double total);

        public static string fetch (string url) throws Error {
            var session = new Soup.SessionSync ();
            var message = new Soup.Message ("GET", url);
            message.request_headers.append ("User-Agent", "curl/7.54");
            session.send_message (message);
            if (message.status_code != 200) {
                throw new IOError.FAILED ("Request failed (status " + message.status_code.to_string () + ")");
            }
            return (string) message.response_body.data;
        }

        public static void download (string url, FileStream file, ProgressCallback? progress = null, Cancellable? cancellable = null) throws Error {
            var session = new Soup.SessionSync ();
            var message = new Soup.Message ("GET", url);
            message.request_headers.append ("User-Agent", "curl/7.54");

            session.send_message (message);
            if (message.status_code != 200) {
                throw new IOError.FAILED ("Request failed (status " + message.status_code.to_string () + ")");
            }
            if (message.response_body.data != null && message.response_body.length > 0) {
                file.write (message.response_body.data, (size_t) message.response_body.length);
            }
            debug ("File saved.");
        }
    }

    public delegate void ExtractProgressCallback (double now, double total);

    public static void extract_zip (FileStream file, string dest_dir, ExtractProgressCallback? progress = null, Cancellable? cancellable = null) throws Error {
        file.seek (0, FileSeek.SET);
        int fd = file.fileno ();
        var a = new Archive.Read ();
        a.support_format_zip ();
        a.support_filter_all ();

        var ext = new Archive.WriteDisk ();
        ext.set_options (Archive.ExtractFlags.TIME | Archive.ExtractFlags.PERM | Archive.ExtractFlags.ACL | Archive.ExtractFlags.FFLAGS);
        ext.set_standard_lookup ();

        int r = a.open_fd (fd, 10240);
        if (r != Archive.Result.OK) {
            throw new IOError.FAILED ("Failed to open zip: " + a.error_string ());
        }

        unowned Archive.Entry entry;
        while (a.next_header (out entry) == Archive.Result.OK) {
            var current_file = Path.build_filename (dest_dir, entry.pathname ());
            entry.set_pathname (current_file);
            r = ext.write_header (entry);
            if (r == Archive.Result.OK && entry.size () > 0) {
                unowned uint8[] buff;
                Archive.int64_t offset;
                while (a.read_data_block (out buff, out offset) == Archive.Result.OK) {
                    ext.write_data_block (buff, offset);
                }
            }
            ext.finish_entry ();
        }
        a.close ();
        ext.close ();
    }

    public static void delete_tree (File dir) throws Error {
        if (!dir.query_exists ()) return;
        var enumerator = dir.enumerate_children ("standard::name,standard::type", FileQueryInfoFlags.NOFOLLOW_SYMLINKS);
        FileInfo info;
        while ((info = enumerator.next_file ()) != null) {
            var child = dir.get_child (info.get_name ());
            if (info.get_file_type () == GLib.FileType.DIRECTORY) {
                delete_tree (child);
            } else {
                child.delete ();
            }
        }
        dir.delete ();
    }

    public delegate void SettingsChangedCallback ();

    public static void bind_settings<T> (KeyFile kf, string section, string key, GLib.Object obj, string prop, SettingsChangedCallback? cb = null) {
        try {
            if (typeof (T) == typeof (string)) {
                var val = kf.get_string (section, key);
                obj.set (prop, val);
            } else if (typeof (T) == typeof (double) || typeof (T) == typeof (double?)) {
                var val = kf.get_double (section, key);
                obj.set (prop, val);
            } else if (typeof (T) == typeof (bool) || typeof (T) == typeof (bool?)) {
                var val = kf.get_boolean (section, key);
                obj.set (prop, val);
            } else if (typeof (T) == typeof (int) || typeof (T) == typeof (int?)) {
                var val = kf.get_integer (section, key);
                obj.set (prop, val);
            }
        } catch (Error e) {
            // keep default
        }

        obj.notify[prop].connect (() => {
            try {
                GLib.Value val = GLib.Value (typeof (T));
                obj.get_property (prop, ref val);
                if (typeof (T) == typeof (string)) {
                    kf.set_string (section, key, val.get_string ());
                } else if (typeof (T) == typeof (double) || typeof (T) == typeof (double?)) {
                    kf.set_double (section, key, val.get_double ());
                } else if (typeof (T) == typeof (bool) || typeof (T) == typeof (bool?)) {
                    kf.set_boolean (section, key, val.get_boolean ());
                } else if (typeof (T) == typeof (int) || typeof (T) == typeof (int?)) {
                    kf.set_integer (section, key, val.get_int ());
                }
            } catch (Error e) {
                warning ("bind_settings error: %s", e.message);
            }
            if (cb != null) {
                cb ();
            }
        });
    }

}
