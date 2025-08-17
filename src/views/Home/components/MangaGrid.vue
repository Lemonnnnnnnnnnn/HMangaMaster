<template>
    <div class="h-screen overflow-auto">
        <div class="flex flex-wrap gap-4 p-8">
            <MangaCard v-for="manga in mangas" :key="manga.path" :manga="manga" />
        </div>
    </div>
</template>

<script setup lang="ts">
import { MangaCard } from ".";
import { useHomeStore } from "../stores/homeStore";

const homeStore = useHomeStore();
const { mangas } = homeStore;
</script>

<style scoped></style>