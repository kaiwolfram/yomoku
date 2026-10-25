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

private fun providerCount(): Int {
    val field = Class.forName("app.yomoku.nativebridge.DictionaryProviderMap")
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

fun main(args: Array<String>) {
    verifyTextBoundaries(args.single())
    verifyDocumentBridge()
    verifyReaderWordBridge()
    val deadline = System.nanoTime() + 10_000_000_000L
    while (listenerCount() != 0 && System.nanoTime() < deadline) Thread.sleep(5)
    check(listenerCount() == 0)
    val listener = Listener()
    NativeBridge(":memory:", listener, HostDictionary()).use { bridge ->
        check(listenerCount() == 1)
        NativeBridge(":memory:", Listener(), HostDictionary()).use { independent ->
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
            for (event in listOf(Event.OpenSettings, Event.CloseSettings)) {
                val bytes = event.bincodeSerialize()
                check(Event.bincodeDeserialize(bytes) == event)
                val requests = Requests.bincodeDeserialize(bridge.update(bytes))
                check(requests.value.single().effect is ShellEffect.Render)
                val view = ViewModel.bincodeDeserialize(bridge.view())
                check(view.settingsOpen == (event is Event.OpenSettings))
                check(view.destination == Destination.LIBRARY)
            }
            expectFailure<FfiError.InvalidResponse> { bridge.resolve(UInt.MAX_VALUE, byteArrayOf()) }
            check(listener.calls == 0) // Navigation remains synchronous.
        }
        check(listenerCount() == 1)
    }
    check(listenerCount() == 0)

    repeat(100) {
        val bridge = NativeBridge(":memory:", Listener(), HostDictionary())
        check(ViewModel.bincodeDeserialize(bridge.view()).destination == Destination.LIBRARY)
        bridge.close()
        bridge.close()
        check(listenerCount() == 0)
        expectFailure<IllegalStateException> { bridge.view() }
        expectFailure<IllegalStateException> { bridge.update(Event.SelectDestination(Destination.NOSTR).bincodeSerialize()) }
        expectFailure<IllegalStateException> { bridge.resolve(0u, byteArrayOf()) }
    }
    val providerDeadline = System.nanoTime() + 10_000_000_000L
    while (providerCount() != 0 && System.nanoTime() < providerDeadline) Thread.sleep(5)
    check(providerCount() == 0)
    println("PASS: Kotlin/JNI navigation, codecs, typed errors, isolation and 100 handle lifecycles")
}
