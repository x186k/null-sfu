rand = 'my rand is ' + Math.floor(Math.random() * 100)
console.log(rand)
a = new RTCPeerConnection()
a.onconnectionstatechange = () => console.log("a state:" + a.connectionState)
a.createDataChannel("dc").onopen = (ev) => ev.target.send(rand)
a.ondatachannel = (ev) => {
    ev.channel.onmessage = (e) => console.log("xdc got msg: " + e.data)
    ev.channel.onopen = () => console.log("xdc is open")
}
await a.setLocalDescription()
await new Promise((r) => setTimeout(r, 0)); // ice canidate settling

fo = { method: "POST", body: a.localDescription.sdp }
r = await fetch("http://localhost:3000/roomkey-xyz", fo)
console.log("fetch status", r.status)
if (r.status === 201) {
    let sdpx = await r.text()
    console.debug("got N line answer:", sdpx.split(/\r\n|\r|\n/).length)
    await a.setRemoteDescription({ type: "answer", sdp: sdpx })
}