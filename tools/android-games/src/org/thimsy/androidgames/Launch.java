package org.thimsy.androidgames;

import android.app.Activity;
import android.net.Uri;
import android.os.Bundle;
import android.util.Log;

import java.io.File;

/**
 * Starts the game a Nano game file names. Nano passes the file as a storage
 * document URI ("content://com.android.externalstorage.documents/tree/.../
 * document/primary%3AROMs%2Fandroidgames%2F<name>.app"), or with
 * "-e path <file>".
 */
public class Launch extends Activity {
    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        File game = gameFile();
        String pkg = game == null ? null : Games.packageOf(game);
        if (pkg != null && !pkg.isEmpty()) {
            Games.launch(this, pkg);
        } else {
            Log.w(Games.TAG, "no game in " + getIntent());
            new Thread(() -> Games.sync(getApplicationContext())).start();
        }
        finish();
    }

    private File gameFile() {
        String path = getIntent().getStringExtra("path");
        if (path != null) return new File(path);
        Uri uri = getIntent().getData();
        if (uri == null) return null;
        if ("file".equals(uri.getScheme())) return new File(uri.getPath());
        // The document ID is the last path segment: "<volume>:<path on it>".
        String doc = uri.getLastPathSegment();
        if (doc == null || !doc.contains(":")) return null;
        String volume = doc.substring(0, doc.indexOf(':'));
        String rel = doc.substring(doc.indexOf(':') + 1);
        String root = "primary".equals(volume) ? "/storage/emulated/0" : "/storage/" + volume;
        return new File(root, rel);
    }
}
