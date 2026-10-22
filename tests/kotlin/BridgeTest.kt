package app.yomoku.tests

import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.Destination
import app.yomoku.shared.Event
import app.yomoku.shared.Requests
import app.yomoku.shared.ShellEffect
import app.yomoku.shared.ViewModel

private class Listener : EffectsListener {
    var calls = 0

    override fun onEffects(requests: ByteArray) { calls++ }
    override fun onError(error: FfiError) { calls++ }
}

// Inspect the pinned generator's registry to verify native destruction releases
// the foreign callback. This avoids timing-dependent garbage collection checks.
private fun listenerCount(): Int {
    val field = Class.forName("app.yomoku.nativebridge.EffectsListenerMap")
        .getDeclaredField("map")
    field.isAccessible = true
    return (field.get(null) as Map<*, *>).size
}

private inline fun <reified T : Throwable> expectFailure(block: () -> Unit) {
    try {
        block()
    } catch (error: Throwable) {
        check(error is T) { "Expected ${T::class}, got $error" }
        return
    }
    error("Expected ${T::class}")
}

fun main() {
    check(listenerCount() == 0)
    val listener = Listener()
    NativeBridge(listener).use { bridge ->
        check(listenerCount() == 1)
        NativeBridge(Listener()).use { independent ->
            check(listenerCount() == 2)
            check(ViewModel.bincodeDeserialize(bridge.view()).destination == Destination.LIBRARY)
            for (destination in Destination.entries + Destination.LIBRARY) {
                val event = Event.SelectDestination(destination)
                val eventBytes = event.bincodeSerialize()
                check(Event.bincodeDeserialize(eventBytes) == event)
                val requestBytes = bridge.update(eventBytes)
                val requests = Requests.bincodeDeserialize(requestBytes)
                check(requests.value.single().effect is ShellEffect.Render)
                check(requests.bincodeSerialize().contentEquals(requestBytes))
                val viewBytes = bridge.view()
                val view = ViewModel.bincodeDeserialize(viewBytes)
                check(view.destination == destination)
                check(view.bincodeSerialize().contentEquals(viewBytes))
                check(ViewModel.bincodeDeserialize(independent.view()).destination == Destination.LIBRARY)
                expectFailure<FfiError.InvalidResponse> {
                    bridge.resolve(requests.value.single().id, byteArrayOf())
                }
            }
            for (badEvent in listOf(byteArrayOf(), byteArrayOf(0), ByteArray(8) { -1 })) {
                expectFailure<FfiError.InvalidEvent> { bridge.update(badEvent) }
                check(ViewModel.bincodeDeserialize(bridge.view()).destination == Destination.LIBRARY)
            }
            expectFailure<FfiError.InvalidResponse> { bridge.resolve(UInt.MAX_VALUE, byteArrayOf()) }
            check(listener.calls == 0) // Navigation is synchronous; async delivery is a later test.
        }
        check(listenerCount() == 1)
    }
    check(listenerCount() == 0)

    repeat(100) {
        val bridge = NativeBridge(Listener())
        check(ViewModel.bincodeDeserialize(bridge.view()).destination == Destination.LIBRARY)
        bridge.close()
        bridge.close()
        check(listenerCount() == 0)
        expectFailure<IllegalStateException> { bridge.view() }
        expectFailure<IllegalStateException> { bridge.update(Event.SelectDestination(Destination.NOSTR).bincodeSerialize()) }
        expectFailure<IllegalStateException> { bridge.resolve(0u, byteArrayOf()) }
    }
    println("PASS: Kotlin/JNI navigation, codecs, typed errors, isolation and 100 handle lifecycles")
}
