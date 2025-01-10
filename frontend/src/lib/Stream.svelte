<script>
  import { onMount } from "svelte";
  import GstWebRTCAPI from "../gstwebrtc-api/gstwebrtc-api";

  let api = $state(null);
  let src = $state(null);
  let session = $state(null);

  	/** @type {HTMLVideoElement} */
  let video;

  onMount(() => {
    const signalingProtocol = window.location.protocol.startsWith("https")
      ? "wss"
      : "ws";
    const gstWebRTCConfig = {
      meta: { name: `WebClient-${Date.now()}` },
      signalingServerUrl: `${signalingProtocol}://${window.location.hostname}:8443`,
      iceServers: [], // We're directly connecting to the AP, so no need for ICE servers
    };

    api = new GstWebRTCAPI(gstWebRTCConfig);

    let listener = {
      producerAdded: (producer) => {
        session = api.createConsumerSession(producer.id);
        session.mungeStereoHack = true; // cargo culting here
        session.addEventListener("error", (event) => {
          console.error("Session error", event);
        });
        session.addEventListener("closed", (event) => {
          console.log("Session closed", event);
          video.srcObject = null;
        });
        session.addEventListener("streamsChanged", () => {
          if (session.streams.length > 0) {
            video.srcObject = session.streams[0];
            // video.play().catch(console.warn);
          }
        })
        session.connect()
      },
      producerRemoved: (producer) => {
        if (session) {
          video.srcObject = null;
          session.close();
          session = null;
        }
      },
    };

    api.registerProducersListener(listener);
    for (const producer of api.getAvailableProducers()) {
      listener.producerAdded(producer);
    }
  });
</script>

<!-- svelte-ignore a11y_media_has_caption -->
{#if api}
  <video controls bind:this={video}> </video>
{:else}
  <p>Waiting for API to initialize...</p>
{/if}
