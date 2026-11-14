package com.android.server.policy;

import org.json.JSONArray;
import org.json.JSONObject;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;

/**
 * Runs ThimsyDeviceKeys.resolve over a cases file ({"table": path relative to it, "cases": [{"props":
 * {...}}, ...], "back_release": [{"shared", "used", "held"}, ...]}) and prints {"resolved": [...],
 * "back_release": [...]}, in the cases' order. "table": null resolves with no table.
 */
public final class DeviceKeysCheck {
    public static void main(String[] args) throws Exception {
        Path casesPath = Paths.get(args[0]);
        JSONObject doc = new JSONObject(read(casesPath));
        String table = doc.isNull("table") ? null
                : read(casesPath.resolveSibling(doc.getString("table")));
        JSONArray cases = doc.getJSONArray("cases");
        JSONArray out = new JSONArray();
        for (int i = 0; i < cases.length(); i++) {
            JSONObject props = cases.getJSONObject(i).optJSONObject("props");
            final JSONObject p = props == null ? new JSONObject() : props;
            ThimsyDeviceKeys k = table == null ? ThimsyDeviceKeys.builtin()
                    : ThimsyDeviceKeys.resolve(table, name -> p.optString(name));
            JSONObject r = new JSONObject();
            r.put("id", k.id);
            JSONArray fl = new JSONArray(), fa = new JSONArray(), gp = new JSONArray(), bl = new JSONArray(),
                    ba = new JSONArray();
            for (ThimsyDeviceKeys.Key key : k.function) {
                fl.put(key.linux);
                fa.put(key.android);
                gp.put(key.gamepadOnly);
            }
            for (ThimsyDeviceKeys.Key key : k.back) {
                bl.put(key.linux);
                ba.put(key.android);
            }
            r.put("function_linux", fl);
            r.put("function_android", fa);
            r.put("gamepad_only", gp);
            r.put("back_linux", bl);
            r.put("back_android", ba);
            r.put("shared_back", k.sharedBack);
            out.put(r);
        }
        JSONArray release = new JSONArray();
        JSONArray rc = doc.optJSONArray("back_release");
        for (int i = 0; rc != null && i < rc.length(); i++) {
            JSONObject c = rc.getJSONObject(i);
            int r = ThimsyDeviceKeys.backRelease(c.getBoolean("shared"), c.getBoolean("used"),
                    c.getLong("held"));
            release.put(r == ThimsyDeviceKeys.BACK_RELEASE_BACK ? "back"
                    : r == ThimsyDeviceKeys.BACK_RELEASE_QUICK_MENU ? "quick_menu" : "nothing");
        }
        JSONObject result = new JSONObject();
        result.put("resolved", out);
        result.put("back_release", release);
        System.out.println(result.toString());
    }

    private static String read(Path p) throws Exception {
        return new String(Files.readAllBytes(p), StandardCharsets.UTF_8);
    }
}
