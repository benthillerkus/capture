<script>
  import Stream from "./lib/Stream.svelte";
  import Controls from "./lib/Controls.svelte";
  import Record from "./lib/Record.svelte";
    import Gallery from "./lib/Gallery.svelte";
</script>

<svelte:head>
  <title>stereokamerasteuerung</title>
</svelte:head>

<main>
  <h1>stereokamerasteuerung</h1>

  <div class="card">
    <Stream></Stream>
  </div>

  <div class="card">
    <Controls></Controls>
  </div>

  <div class="card">
    <Gallery></Gallery>
  </div>

  <Record></Record>

  <a href="https://github.com/benthillerkus/capture">code</a>
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    justify-content: flex-start;
    height: 100%;
    width: 100%;
  }

  a {
    margin-top: 32px;
    text-align: center;
    color: grey;
    margin: 4px;
  }
</style>
