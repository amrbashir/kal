<template>
  <router-view></router-view>
</template>

<style>
* {
  padding: 0;
  margin: 0;
  box-sizing: border-box;

  scrollbar-color: var(--neutral-fill-strong-rest) var(--neutral-fill-layer-rest);
  scrollbar-width: thin;
}

html,
body {
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  font-family:
    "Segoe UI Variable",
    "Segoe UI",
    -apple-system,
    BlinkMacSystemFont,
    Roboto,
    Oxygen,
    Ubuntu,
    Cantarell,
    "Open Sans",
    "Helvetica Neue",
    sans-serif;
}

.fade-leave-active,
.fade-enter-active {
  transition: opacity 0.3s ease-out;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
.slide-fade-leave-active,
.slide-fade-enter-active {
  transition: all 0.3s ease-out;
}
.slide-fade-enter-from,
.slide-fade-leave-to {
  transform: translateX(20px);
  opacity: 0;
}
</style>
