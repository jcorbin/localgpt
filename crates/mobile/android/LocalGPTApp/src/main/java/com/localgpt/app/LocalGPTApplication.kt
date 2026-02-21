package com.localgpt.app

import android.app.Application
import uniffi.localgpt_mobile.*

class LocalGPTApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        // UniFFI initialization if needed (usually automatic on library load)
    }
}
