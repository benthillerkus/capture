<script>
  import { onMount } from "svelte";

  const API_HOST = import.meta.env.VITE_API_HOST;

  let content = $state([]);
  $inspect(content);

  async function fetchGallery() {
    const response = await fetch(`${API_HOST}/api/gallery`);
    content = await response.json();
  }

  onMount(() => {
    fetchGallery();
    setInterval(fetchGallery, 5000);
  });
</script>

<h2>Gallery</h2>
<div class="gallery">
  {#each content as item}
    <!-- svelte-ignore a11y_media_has_caption -->
    <div>
      <video src={`/gallery/${item}`} controls> </video>
      <a href={`/gallery/${item}`}>{item} (download)</a>
    </div>
  {/each}
</div>

<style>
  .gallery {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 32px 16px;
  }

  video {
    width: 100%;
    height: auto;
    border-radius: 32px;
  }
</style>
