<script setup lang="ts">
defineProps<{
  message: string
}>()
</script>

<template>
  <main>
    <h1>Askama + Inertia</h1>
    <p>{{ message }}</p>
  </main>
</template>
