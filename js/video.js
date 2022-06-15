document.documentElement.innerHTML = ""
const video = document.createElement("video")
video.autoplay = true
video.controls = true
video.muted = false
video.width = 640
video.height = 480
video.visible = true
//video.poster = "https://peach.blender.org/wp-content/uploads/title_anouncement.jpg?x11217"
document.documentElement.appendChild(video)
gum = await navigator.mediaDevices.getUserMedia({ audio: true, video: true })
//tx.sender.setStreams(x)
// a.addTransceiver(x.getVideoTracks()[0], { 'direction': 'sendonly' })
a = new RTCPeerConnection()
a.addTrack(gum.getVideoTracks()[0])
a.ontrack = (e) => { console.log(11, e); video.srcObject = new MediaStream([e.track]) }
a.onconnectionstatechange = () => console.log("a state:" + a.connectionState)
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