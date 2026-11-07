package org.gammaosnextimproved.androidgames;

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
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;

/**
 * Keeps /sdcard/ROMs/androidgames in step with the installed games: one
 * "<game name>.app" file per game, holding its package name, and its icon as
 * the game's art in the ES-DE media folder, where Nano looks for it. A game's
 * file keeps its name once made, since Nano keys favourites, recents and
 * collections by it, and art the user put there (scraped covers) is left alone.
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

    /** The package a game file names, or null. */
    static String packageOf(File game) {
        try (BufferedReader r = new BufferedReader(new FileReader(game))) {
            String line = r.readLine();
            return line == null ? null : line.trim();
        } catch (IOException e) {
            return null;
        }
    }

    static synchronized void sync(Context context) {
        PackageManager pm = context.getPackageManager();
        // The art files this app wrote, with the modification time it left them at.
        SharedPreferences ours = context.getSharedPreferences("art", Context.MODE_PRIVATE);
        if (!ours.getBoolean("adopted", false)) adoptArt(pm, ours);
        Set<String> exclude = readExclude();
        exclude.add(context.getPackageName());

        Map<String, String> games = new HashMap<>();   // package -> file name
        for (ApplicationInfo app : pm.getInstalledApplications(0)) {
            boolean game = app.category == ApplicationInfo.CATEGORY_GAME
                    || (app.flags & ApplicationInfo.FLAG_IS_GAME) != 0;
            if (!game || exclude.contains(app.packageName)
                    || pm.getLaunchIntentForPackage(app.packageName) == null) {
                continue;
            }
            games.put(app.packageName, fileName(pm.getApplicationLabel(app).toString()));
        }

        if (!ROMS.isDirectory() && !ROMS.mkdirs()) {
            Log.w(TAG, "can't create " + ROMS);
            return;
        }
        Set<String> listed = new HashSet<>();
        File[] files = ROMS.listFiles();
        if (files != null) {
            for (File f : files) {
                if (!f.getName().endsWith(EXT)) continue;
                String pkg = packageOf(f);
                if (pkg == null) continue;
                if (!isInstalled(pm, pkg)) {
                    delete(ours, f);
                    continue;
                }
                listed.add(pkg);
                String label = games.get(pkg);
                if (label == null) continue;
                String name = baseName(f);
                // Nano skips dot files, so a name from an older version that starts
                // with a dot never showed up: give it the current one.
                if (name.startsWith(".")) {
                    String renamed = unique(label);
                    if (f.renameTo(new File(ROMS, renamed + EXT))) {
                        deleteArt(ours, name);
                        name = renamed;
                    }
                }
                writeArt(pm, ours, pkg, name);
            }
        }
        int added = 0;
        for (Map.Entry<String, String> g : games.entrySet()) {
            if (listed.contains(g.getKey())) continue;
            String name = unique(g.getValue());
            if (write(new File(ROMS, name + EXT), g.getKey())) {
                writeArt(pm, ours, g.getKey(), name);
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
    private static void writeArt(PackageManager pm, SharedPreferences ours, String pkg, String name) {
        long updated;
        Drawable icon;
        try {
            updated = pm.getPackageInfo(pkg, 0).lastUpdateTime;
            icon = pm.getApplicationIcon(pkg);
        } catch (PackageManager.NameNotFoundException e) {
            return;
        }
        Bitmap bitmap = null;
        SharedPreferences.Editor written = ours.edit();
        for (String dir : ART) {
            File folder = new File(MEDIA, dir);
            File art = new File(folder, name + ".png");
            if (art.exists() && (!isOurs(ours, dir, art) || art.lastModified() >= updated)) continue;
            if (!folder.isDirectory() && !folder.mkdirs()) continue;
            if (bitmap == null) bitmap = render(icon);
            // Written whole to a temporary file first, so a failed write never
            // leaves a broken PNG that looks current.
            File tmp = new File(folder, "." + art.getName() + ".tmp");
            boolean ok;
            try (FileOutputStream out = new FileOutputStream(tmp)) {
                ok = bitmap.compress(Bitmap.CompressFormat.PNG, 100, out);
                out.getFD().sync();
            } catch (IOException e) {
                ok = false;
            }
            if (!ok || !tmp.renameTo(art)) {
                Log.w(TAG, "can't write art for " + pkg);
                tmp.delete();
                continue;
            }
            written.putLong(dir + "/" + art.getName(), art.lastModified());
        }
        written.apply();
        if (bitmap != null) bitmap.recycle();
    }

    private static Bitmap render(Drawable icon) {
        int size = 512;
        Bitmap bitmap = Bitmap.createBitmap(size, size, Bitmap.Config.ARGB_8888);
        icon.setBounds(0, 0, size, size);
        icon.draw(new Canvas(bitmap));
        return bitmap;
    }

    /**
     * The first test build kept no record of the art it wrote. Its art is the
     * game's icon rendered as writeArt renders it, so a file that is exactly
     * that image counts as this app's; any other file (a scraped or chosen
     * cover) is left as the user's.
     */
    private static void adoptArt(PackageManager pm, SharedPreferences ours) {
        SharedPreferences.Editor written = ours.edit();
        File[] files = ROMS.listFiles();
        if (files != null) {
            for (File f : files) {
                if (!f.getName().endsWith(EXT)) continue;
                String pkg = packageOf(f);
                byte[] png = pkg == null ? null : iconPng(pm, pkg);
                if (png == null) continue;
                for (String dir : ART) {
                    File art = new File(new File(MEDIA, dir), baseName(f) + ".png");
                    try {
                        if (art.isFile() && art.length() == png.length
                                && Arrays.equals(readAll(art, png.length), png)) {
                            written.putLong(dir + "/" + art.getName(), art.lastModified());
                        }
                    } catch (IOException e) {
                        Log.w(TAG, "can't read " + art);
                    }
                }
            }
        }
        written.putBoolean("adopted", true).apply();
    }

    private static byte[] readAll(File f, int length) throws IOException {
        byte[] data = new byte[length];
        try (FileInputStream in = new FileInputStream(f)) {
            int n = 0;
            for (int r; n < length && (r = in.read(data, n, length - n)) > 0; ) n += r;
            return n == length ? data : null;
        }
    }

    private static byte[] iconPng(PackageManager pm, String pkg) {
        Drawable icon;
        try {
            icon = pm.getApplicationIcon(pkg);
        } catch (PackageManager.NameNotFoundException e) {
            return null;
        }
        Bitmap bitmap = render(icon);
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        boolean ok = bitmap.compress(Bitmap.CompressFormat.PNG, 100, out);
        bitmap.recycle();
        return ok ? out.toByteArray() : null;
    }

    private static boolean isOurs(SharedPreferences ours, String dir, File art) {
        return ours.getLong(dir + "/" + art.getName(), -1) == art.lastModified();
    }

    private static String baseName(File game) {
        return game.getName().substring(0, game.getName().length() - EXT.length());
    }

    private static void delete(SharedPreferences ours, File game) {
        game.delete();
        deleteArt(ours, baseName(game));
    }

    /** Deletes the game's art that this app wrote. */
    private static void deleteArt(SharedPreferences ours, String name) {
        SharedPreferences.Editor written = ours.edit();
        for (String dir : ART) {
            File art = new File(new File(MEDIA, dir), name + ".png");
            if (isOurs(ours, dir, art)) art.delete();
            written.remove(dir + "/" + art.getName());
        }
        written.apply();
    }
}
