package org.gammaosnextimproved.androidgames;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;

/** Re-lists the games after boot and whenever an app is installed or removed. */
public class SyncReceiver extends BroadcastReceiver {
    @Override
    public void onReceive(Context context, Intent intent) {
        PendingResult result = goAsync();
        Context app = context.getApplicationContext();
        new Thread(() -> {
            try {
                Games.sync(app);
            } finally {
                result.finish();
            }
        }).start();
    }
}
