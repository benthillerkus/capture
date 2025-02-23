<script>
  import { onMount } from "svelte";
  import GstWebRTCAPI from "../gstwebrtc-api/gstwebrtc-api";

  let api = $state(null);
  let src = $state(null);
  let session = $state(null);

  /** @type {HTMLVideoElement} */
  let video = $state(null);
  let interacted = $state(false);

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
            if (interacted) {
              video.play().catch(console.warn);
            }
          }
        });
        session.connect();
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

<div>
  <!-- svelte-ignore a11y_media_has_caption -->
  {#if session}
    {#if !interacted}
      <p>Connected.<br>Click to watch camera feed</p>
    {/if}
    <video controls bind:this={video} onplay={(_) => (interacted = true)}>
    </video>
  {:else}
    <p>Connecting to Camera...</p>
  {/if}
</div>

<style>
  div {
    display: grid;
    grid-template: 1fr / 1fr;
    justify-content: center;
    align-items: center;
    text-align: center;
  }

  div > * {
    grid-row: 1;
    grid-column: 1;
  }

  video {
    width: 100%;
    max-width: 100%;
    height: auto;
    border-radius: 32px 32px 0px 0px;
  }
</style>
