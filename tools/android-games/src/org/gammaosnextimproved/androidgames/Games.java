package org.gammaosnextimproved.androidgames;

import android.content.Context;
import android.content.Intent;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageManager;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.drawable.Drawable;
import android.util.Log;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileOutputStream;
import java.io.FileReader;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;

/**
 * Keeps /sdcard/ROMs/androidgames in step with the installed games: one
 * "<game name>.app" file per game, holding its package name, and its icon as
 * the game's art in the ES-DE media folder, where Nano looks for it.
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
                if (isInstalled(pm, pkg)) {
                    listed.add(pkg);
                } else {
                    delete(f);
                }
            }
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

    private static void writeArt(PackageManager pm, String pkg, String name) {
        Drawable icon;
        try {
            icon = pm.getApplicationIcon(pkg);
        } catch (PackageManager.NameNotFoundException e) {
            return;
        }
        int size = 512;
        Bitmap bitmap = Bitmap.createBitmap(size, size, Bitmap.Config.ARGB_8888);
        icon.setBounds(0, 0, size, size);
        icon.draw(new Canvas(bitmap));
        for (String dir : ART) {
            File folder = new File(MEDIA, dir);
            if (!folder.isDirectory() && !folder.mkdirs()) continue;
            try (FileOutputStream out = new FileOutputStream(new File(folder, name + ".png"))) {
                bitmap.compress(Bitmap.CompressFormat.PNG, 100, out);
            } catch (IOException e) {
                Log.w(TAG, "can't write art for " + pkg, e);
            }
        }
        bitmap.recycle();
    }

    private static void delete(File game) {
        String base = game.getName().substring(0, game.getName().length() - EXT.length());
        game.delete();
        for (String dir : ART) {
            new File(new File(MEDIA, dir), base + ".png").delete();
        }
    }
}
