<script>
  const API_HOST = import.meta.env.VITE_API_HOST;

  import { onMount } from "svelte";

  let isRecording = $state(false);

  onMount(() => {
    fetch(`${API_HOST}/api/state`, {
      method: "GET",
    })
      .then((response) => {
        return response.json();
      })
      .then((body) => {
        isRecording = body[0] == "Capture";
      });
  });

  async function record() {
    isRecording = !isRecording;
    let response = await fetch(`${API_HOST}/api/record`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(isRecording),
    });

    let body = (await response.json())[0];

    if (body == "Capture") {
      isRecording = true;
    }
    if (body == "Livefeed") {
      isRecording = false;
    }
  }
</script>

<button onclick={record}> {!isRecording ? "Record" : "Stop"} </button>

<style>
  button {
    position: absolute;
    bottom: 32px;
    right: 32px;
    border-radius: 100%;
    aspect-ratio: 1;
  }
</style>
