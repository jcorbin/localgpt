package com.localgpt.app

import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.localgpt_mobile.*
import java.io.File
import java.util.*

data class Message(
    val id: String = UUID.randomUUID().toString(),
    val text: String,
    val isUser: Boolean,
    val timestamp: Long = System.currentTimeMillis()
)

class ChatViewModel : ViewModel() {
    private var client: LocalGPTClient? = null
    
    val messages = mutableStateListOf<Message>()
    val isThinking = mutableStateOf(false)
    val errorMessage = mutableStateOf<String?>(null)

    fun initialize(dataDir: File) {
        if (client != null) return
        
        viewModelScope.launch(Dispatchers.IO) {
            try {
                val appDir = File(dataDir, "LocalGPT")
                if (!appDir.exists()) appDir.mkdirs()
                
                val newClient = LocalGPTClient(appDir.absolutePath)
                client = newClient
                
                if (newClient.isBrandNew()) {
                    withContext(Dispatchers.Main) {
                        messages.add(Message(text = getWelcomeMessage(), isUser = false))
                    }
                }
            } catch (e: Exception) {
                withContext(Dispatchers.Main) {
                    errorMessage.value = "Init error: ${e.localizedMessage}"
                }
            }
        }
    }

    fun sendMessage(text: String) {
        val userMsg = Message(text = text, isUser = true)
        messages.add(userMsg)
        
        isThinking.value = true
        
        viewModelScope.launch(Dispatchers.IO) {
            try {
                val response = client?.chat(text) ?: "Error: Client not initialized"
                withContext(Dispatchers.Main) {
                    isThinking.value = false
                    messages.add(Message(text = response, isUser = false))
                }
            } catch (e: Exception) {
                withContext(Dispatchers.Main) {
                    isThinking.value = false
                    errorMessage.value = "Chat error: ${e.localizedMessage}"
                }
            }
        }
    }

    fun resetSession() {
        viewModelScope.launch(Dispatchers.IO) {
            try {
                client?.newSession()
                withContext(Dispatchers.Main) {
                    messages.clear()
                    if (client?.isBrandNew() == true) {
                        messages.add(Message(text = getWelcomeMessage(), isUser = false))
                    }
                }
            } catch (e: Exception) {
                withContext(Dispatchers.Main) {
                    errorMessage.value = "Reset error: ${e.localizedMessage}"
                }
            }
        }
    }
}
