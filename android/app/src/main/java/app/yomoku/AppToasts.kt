package app.yomoku

import android.content.Context
import android.widget.Toast

internal interface ToastMessage {
    fun show()

    fun cancel()
}

/** One owner for transient app feedback; a new message replaces the visible one. */
internal class AppToasts(private val create: (String) -> ToastMessage) {
    constructor(
        context: Context
    ) : this({ message ->
        val toast = Toast.makeText(context, message, Toast.LENGTH_SHORT)
        object : ToastMessage {
            override fun show() = toast.show()

            override fun cancel() = toast.cancel()
        }
    })

    private var current: ToastMessage? = null

    fun show(message: String) {
        cancel()
        current = create(message).also { it.show() }
    }

    fun cancel() {
        current?.cancel()
        current = null
    }
}
