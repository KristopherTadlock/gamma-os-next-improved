package org.thimsy.androidgames;

import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageManager;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.drawable.Drawable;
import android.util.Log;

import java.io.BufferedReader;
import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.FileReader;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;
import java.util.zip.CRC32;

/**
 * Keeps /sdcard/ROMs/androidgames in step with the installed games: one
 * "<game name>.app" file per game, holding its package name, and its icon as
 * the game's art in the ES-DE media folder, where Nano looks for it. A game's
 * file keeps its name once made, since Nano keys favourites, recents and
 * collections by it. Nano also writes a file for an app added by hand, which
 * is kept, with its icon, while the app is installed and has a launcher
 * activity. The art this app writes carries a PNG text chunk naming
 * it, and only marked art is ever replaced or deleted, so art the user put
 * there (scraped covers) is left alone.
 */
final class Games {
    static final String TAG = "AndroidGames";
    static final File ROMS = new File("/storage/emulated/0/ROMs/androidgames");
    static final File MEDIA = new File("/storage/emulated/0/ES-DE/downloaded_media/androidgames");
    static final String[] ART = {"covers", "miximages"};
    static final String EXT = ".app";
    // The apps first-boot setup installs (emulators and tools), which are not games here.
    static final File EXCLUDE = new File("/system/etc/gammaos/android-games-exclude.txt");

    private Games() {}

    /** The package a game file names, "" for an empty file, or null if it can't be read. */
    static String packageOf(File game) {
        try (BufferedReader r = new BufferedReader(new FileReader(game))) {
            String line = r.readLine();
            return line == null ? "" : line.trim();
        } catch (IOException e) {
            return null;
        }
    }

    static synchronized void sync(Context context) {
        PackageManager pm = context.getPackageManager();
        // Marking the first test build's art; retried on the next sync until every file is done.
        SharedPreferences state = context.getSharedPreferences("art", Context.MODE_PRIVATE);
        if (!state.getBoolean("marked_v1", false) && adoptArt(pm)) {
            state.edit().putBoolean("marked_v1", true).apply();
        }
        Set<String> exclude = readExclude();
        exclude.add(context.getPackageName());

        Map<String, String> games = new HashMap<>();   // package -> file name
        for (ApplicationInfo app : pm.getInstalledApplications(0)) {
            boolean game = app.category == ApplicationInfo.CATEGORY_GAME
                    || (app.flags & ApplicationInfo.FLAG_IS_GAME) != 0;
            if (!game || exclude.contains(app.packageName) || !hasLauncher(pm, app.packageName)) {
                continue;
            }
            games.put(app.packageName, fileName(pm.getApplicationLabel(app).toString()));
        }

        if (!ROMS.isDirectory() && !ROMS.mkdirs()) {
            Log.w(TAG, "can't create " + ROMS);
            return;
        }
        // A game missing from listed gets a new file, so add none unless every existing file was
        // read: an unreadable folder or file (storage still mounting) would otherwise duplicate
        // games. The next sync tries again.
        Set<String> listed = new HashSet<>();
        boolean complete = true;
        File[] files = ROMS.listFiles();
        if (files == null) {
            complete = false;
        } else {
            for (File f : files) {
                if (!f.getName().endsWith(EXT) || f.isDirectory()) continue;   // a folder is never a game file
                String pkg = packageOf(f);
                if (pkg == null) { complete = false; continue; }
                if (pkg.isEmpty()) continue;
                if (!isInstalled(pm, pkg)) {
                    delete(f);
                    continue;
                }
                String label = games.get(pkg);
                if (label == null && !hasLauncher(pm, pkg)) {
                    delete(f);   // an earlier version listed it, but Nano cannot start it
                    continue;
                }
                listed.add(pkg);
                String name = baseName(f);
                // Nano skips dot files, so a name from an older version that starts
                // with a dot never showed up: give it the current one.
                if (label != null && name.startsWith(".")) {
                    String renamed = unique(label);
                    if (f.renameTo(new File(ROMS, renamed + EXT))) {
                        deleteArt(name);
                        name = renamed;
                    }
                }
                writeArt(pm, pkg, name);   // also for an app added by hand in Nano
            }
        }
        if (!complete) {
            Log.w(TAG, "can't read every file in " + ROMS + "; adding no games until the next sync");
            return;
        }
        int added = 0;
        for (Map.Entry<String, String> g : games.entrySet()) {
            if (listed.contains(g.getKey())) continue;
            String name = unique(g.getValue());
            if (write(new File(ROMS, name + EXT), g.getKey())) {
                writeArt(pm, g.getKey(), name);
                added++;
            }
        }
        Log.i(TAG, games.size() + " games, " + added + " added");
    }

    static void launch(Context context, String pkg) {
        Intent intent = context.getPackageManager().getLaunchIntentForPackage(pkg);
        if (intent == null) {
            Log.w(TAG, "no launcher activity in " + pkg);
            return;
        }
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        context.startActivity(intent);
    }

    /**
     * The game has a launcher activity, which is what Nano starts. An app whose
     * only entry is CATEGORY_INFO (which getLaunchIntentForPackage also accepts)
     * is not listed, since Nano could not start it.
     */
    private static boolean hasLauncher(PackageManager pm, String pkg) {
        Intent main = new Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER).setPackage(pkg);
        return !pm.queryIntentActivities(main, 0).isEmpty();
    }

    private static boolean isInstalled(PackageManager pm, String pkg) {
        try {
            pm.getApplicationInfo(pkg, 0);
            return true;
        } catch (PackageManager.NameNotFoundException e) {
            return false;
        }
    }

    private static Set<String> readExclude() {
        Set<String> out = new HashSet<>();
        try (BufferedReader r = new BufferedReader(new FileReader(EXCLUDE))) {
            for (String line; (line = r.readLine()) != null; ) {
                line = line.trim();
                if (!line.isEmpty() && !line.startsWith("#")) out.add(line);
            }
        } catch (IOException e) {
            Log.w(TAG, "can't read " + EXCLUDE);
        }
        return out;
    }

    private static String fileName(String label) {
        String name = label.replaceAll("[\\\\/:*?\"<>|\\p{Cntrl}]", " ").replaceAll("\\s+", " ").trim();
        // Nano skips files whose names start with a dot.
        name = name.replaceFirst("^[.\\s]+", "");
        return name.isEmpty() ? "Game" : name;
    }

    private static String unique(String name) {
        String candidate = name;
        for (int i = 2; new File(ROMS, candidate + EXT).exists(); i++) {
            candidate = name + " (" + i + ")";
        }
        return candidate;
    }

    private static boolean write(File f, String text) {
        try (FileOutputStream out = new FileOutputStream(f)) {
            out.write((text + "\n").getBytes(StandardCharsets.UTF_8));
            return true;
        } catch (IOException e) {
            Log.w(TAG, "can't write " + f, e);
            return false;
        }
    }

    /**
     * Writes the game's icon as its art where there is none, or where this app
     * wrote it before the installed version of the game (an update can change
     * the icon). Art from anywhere else is kept.
     */
    private static void writeArt(PackageManager pm, String pkg, String name) {
        long updated;
        try {
            updated = pm.getPackageInfo(pkg, 0).lastUpdateTime;
        } catch (PackageManager.NameNotFoundException e) {
            return;
        }
        byte[] png = null;
        for (String dir : ART) {
            File art = new File(new File(MEDIA, dir), name + ".png");
            if (art.exists() && (!isOurs(art) || art.lastModified() >= updated)) continue;
            if (png == null && (png = iconPng(pm, pkg)) == null) return;
            publish(art, mark(png));
        }
    }

    /**
     * Writes data to f through a temporary file, so a failed write never leaves
     * a broken PNG in its place. False if f was not written.
     */
    private static boolean publish(File f, byte[] data) {
        File folder = f.getParentFile();
        if (!folder.isDirectory() && !folder.mkdirs()) return false;
        File tmp = new File(folder, "." + f.getName() + ".tmp");
        boolean ok;
        try (FileOutputStream out = new FileOutputStream(tmp)) {
            out.write(data);
            out.getFD().sync();
            ok = true;
        } catch (IOException e) {
            ok = false;
        }
        if (!ok || !tmp.renameTo(f)) {
            Log.w(TAG, "can't write " + f);
            tmp.delete();
            return false;
        }
        return true;
    }

    /** The game's icon at 512px as a PNG, the way the first test build wrote it. */
    private static byte[] iconPng(PackageManager pm, String pkg) {
        Drawable icon;
        try {
            icon = pm.getApplicationIcon(pkg);
        } catch (PackageManager.NameNotFoundException e) {
            return null;
        }
        int size = 512;
        Bitmap bitmap = Bitmap.createBitmap(size, size, Bitmap.Config.ARGB_8888);
        icon.setBounds(0, 0, size, size);
        icon.draw(new Canvas(bitmap));
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        boolean ok = bitmap.compress(Bitmap.CompressFormat.PNG, 100, out);
        bitmap.recycle();
        return ok ? out.toByteArray() : null;
    }

    // A tEXt chunk ("Software" = this package) right after the PNG signature (8
    // bytes) and IHDR, which is always the first chunk and 25 bytes long.
    private static final int MARK_AT = 8 + 25;
    private static final byte[] MARK_CHUNK = textChunk("Software", "org.thimsy.androidgames");

    private static byte[] textChunk(String keyword, String text) {
        byte[] data = (keyword + "\0" + text).getBytes(StandardCharsets.ISO_8859_1);
        ByteBuffer chunk = ByteBuffer.allocate(12 + data.length);
        chunk.putInt(data.length).put("tEXt".getBytes(StandardCharsets.ISO_8859_1)).put(data);
        CRC32 crc = new CRC32();
        crc.update(chunk.array(), 4, 4 + data.length);
        chunk.putInt((int) crc.getValue());
        return chunk.array();
    }

    private static byte[] mark(byte[] png) {
        byte[] marked = new byte[png.length + MARK_CHUNK.length];
        System.arraycopy(png, 0, marked, 0, MARK_AT);
        System.arraycopy(MARK_CHUNK, 0, marked, MARK_AT, MARK_CHUNK.length);
        System.arraycopy(png, MARK_AT, marked, MARK_AT + MARK_CHUNK.length, png.length - MARK_AT);
        return marked;
    }

    /** The art carries this app's mark: this app wrote it. */
    private static boolean isOurs(File art) {
        try {
            byte[] head = readAll(art, MARK_AT + MARK_CHUNK.length);
            return head != null && Arrays.equals(
                    Arrays.copyOfRange(head, MARK_AT, head.length), MARK_CHUNK);
        } catch (IOException e) {
            return false;
        }
    }

    /**
     * The first test build wrote unmarked art. Its art is exactly the game's
     * icon as iconPng renders it, so a file that is that image gets the mark;
     * any other file (a scraped or chosen cover) is left as the user's. False
     * if a file could not be marked, so it is tried again.
     */
    private static boolean adoptArt(PackageManager pm) {
        File[] files = ROMS.listFiles();
        if (files == null) return false;   // not readable yet (storage not mounted): try again
        boolean done = true;
        for (File f : files) {
            if (!f.getName().endsWith(EXT)) continue;
            String pkg = packageOf(f);
            if (pkg == null || pkg.isEmpty()) {
                if (!f.canRead()) done = false;   // try again; an empty file names no game
                continue;
            }
            byte[] png = iconPng(pm, pkg);
            if (png == null) continue;   // not installed: nothing to compare against
            for (String dir : ART) {
                File art = new File(new File(MEDIA, dir), baseName(f) + ".png");
                try {
                    if (art.isFile() && art.length() == png.length
                            && Arrays.equals(readAll(art, png.length), png)) {
                        long modified = art.lastModified();
                        if (publish(art, mark(png))) {
                            art.setLastModified(modified);   // still as old as the icon it shows
                        } else {
                            done = false;
                        }
                    }
                } catch (IOException e) {
                    Log.w(TAG, "can't read " + art);
                    done = false;
                }
            }
        }
        return done;
    }

    /** The first length bytes of f, or null if it is shorter. */
    private static byte[] readAll(File f, int length) throws IOException {
        byte[] data = new byte[length];
        try (FileInputStream in = new FileInputStream(f)) {
            int n = 0;
            for (int r; n < length && (r = in.read(data, n, length - n)) > 0; ) n += r;
            return n == length ? data : null;
        }
    }

    private static String baseName(File game) {
        return game.getName().substring(0, game.getName().length() - EXT.length());
    }

    private static void delete(File game) {
        game.delete();
        deleteArt(baseName(game));
    }

    /** Deletes the game's art that this app wrote. */
    private static void deleteArt(String name) {
        for (String dir : ART) {
            File art = new File(new File(MEDIA, dir), name + ".png");
            if (isOurs(art)) art.delete();
        }
    }
}
