<script lang="ts">
  import DismissIcon from '@fluentui/svg-icons/icons/dismiss_20_regular.svg?no-inline'
  import { appState, tagColors } from '../../appState.svelte'
  import { formatItems } from '../../format'
  import type { SortKey, Theme } from '../../appState.svelte'
  import { tabs } from '../../tabs.svelte'

  const themes: Array<{ value: Theme; label: string }> = [
    { value: 'system', label: 'System' },
    { value: 'light', label: 'Light' },
    { value: 'dark', label: 'Dark' },
  ]

  const sortKeys: Array<{ value: SortKey; label: string }> = [
    { value: 'name', label: 'Name' },
    { value: 'modified', label: 'Date modified' },
    { value: 'type', label: 'Type' },
    { value: 'size', label: 'Size' },
  ]

  let newTagLabel = $state('')

  function addTag() {
    const label = newTagLabel.trim()

    if (!label) return

    appState.createTag(label)
    newTagLabel = ''
  }

  function cycleColor(id: string, current: string) {
    const next = tagColors[(tagColors.indexOf(current as (typeof tagColors)[number]) + 1) % tagColors.length]

    appState.updateTag(id, { color: next })
  }
</script>

<div class="settings">
  <section class="settings__group" aria-labelledby="settings-appearance">
    <h2 id="settings-appearance" class="settings__heading">Appearance</h2>

    <div class="settings__row">
      <div class="settings__label">
        <span>Theme</span>
        <span class="settings__hint">System follows your desktop setting.</span>
      </div>
      <div class="settings__segmented" role="group" aria-label="Theme">
        {#each themes as option (option.value)}
          <button
            class="settings__segment"
            class:settings__segment--on={appState.settings.theme === option.value}
            type="button"
            aria-pressed={appState.settings.theme === option.value}
            onclick={() => appState.update({ theme: option.value })}
          >
            {option.label}
          </button>
        {/each}
      </div>
    </div>
  </section>

  <section class="settings__group" aria-labelledby="settings-files">
    <h2 id="settings-files" class="settings__heading">Files</h2>

    <div class="settings__row">
      <label class="settings__label" for="setting-hidden">
        <span>Show hidden files</span>
        <span class="settings__hint">Items whose name begins with a dot.</span>
      </label>
      <input
        id="setting-hidden"
        class="settings__switch"
        type="checkbox"
        checked={appState.settings.showHidden}
        onchange={(event) => {
          appState.setShowHidden(event.currentTarget.checked)
          tabs.active.reload()
        }}
      />
    </div>

    <div class="settings__row">
      <label class="settings__label" for="setting-sort">
        <span>Default sort</span>
        <span class="settings__hint">Applies to folders you have not sorted yourself.</span>
      </label>
      <select
        id="setting-sort"
        class="settings__select"
        value={appState.settings.defaultSort}
        onchange={(event) => {
          appState.update({ defaultSort: event.currentTarget.value as SortKey })
          tabs.active.reload()
        }}
      >
        {#each sortKeys as key (key.value)}
          <option value={key.value}>{key.label}</option>
        {/each}
      </select>
    </div>

    <div class="settings__row">
      <label class="settings__label" for="setting-descending">
        <span>Sort descending by default</span>
      </label>
      <input
        id="setting-descending"
        class="settings__switch"
        type="checkbox"
        checked={appState.settings.defaultDescending}
        onchange={(event) => {
          appState.update({ defaultDescending: event.currentTarget.checked })
          tabs.active.reload()
        }}
      />
    </div>
  </section>

  <section class="settings__group" aria-labelledby="settings-tags">
    <h2 id="settings-tags" class="settings__heading">Tags</h2>

    <ul class="settings__list">
      {#each appState.tags as tag (tag.id)}
        <li class="settings__item">
          <button class="settings__swatch" type="button" style="background: {tag.color}" aria-label={`Change colour of ${tag.label}`} onclick={() => cycleColor(tag.id, tag.color)}></button>
          <input
            class="settings__input"
            type="text"
            aria-label={`Rename ${tag.label}`}
            value={tag.label}
            onchange={(event) => appState.updateTag(tag.id, { label: event.currentTarget.value.trim() || tag.label })}
          />
          <span class="settings__count">{formatItems(appState.pathsWithTag(tag.id).length)}</span>
          <button class="settings__remove" type="button" aria-label={`Delete ${tag.label}`} onclick={() => appState.removeTag(tag.id)}>
            <span class="masked-icon" style="--icon: url({DismissIcon})" aria-hidden="true"></span>
          </button>
        </li>
      {:else}
        <li class="settings__empty">No tags yet.</li>
      {/each}
    </ul>

    <div class="settings__add">
      <input
        class="settings__input"
        type="text"
        aria-label="New tag name"
        placeholder="New tag name"
        bind:value={newTagLabel}
        onkeydown={(event) => event.key === 'Enter' && addTag()}
      />
      <button class="settings__button" type="button" disabled={!newTagLabel.trim()} onclick={addTag}>Add tag</button>
    </div>
  </section>

  <section class="settings__group" aria-labelledby="settings-pins">
    <h2 id="settings-pins" class="settings__heading">Pinned locations</h2>

    <ul class="settings__list">
      {#each appState.pins as pin (pin.path)}
        <li class="settings__item">
          <button class="settings__pin" type="button" onclick={() => tabs.active.open(pin.path)}>
            <span>{pin.label}</span>
            <span class="settings__path">{pin.path}</span>
          </button>
          <button class="settings__remove" type="button" aria-label={`Unpin ${pin.label}`} onclick={() => appState.unpin(pin.path)}>
            <span class="masked-icon" style="--icon: url({DismissIcon})" aria-hidden="true"></span>
          </button>
        </li>
      {:else}
        <li class="settings__empty">Nothing pinned. Right-click a folder and choose Pin to sidebar.</li>
      {/each}
    </ul>
  </section>

  <section class="settings__group" aria-labelledby="settings-about">
    <h2 id="settings-about" class="settings__heading">About</h2>
    <p class="settings__hint">Pins, tags and settings are stored in <code>~/.config/omafil/state.json</code>.</p>
    {#if appState.error}
      <p class="settings__error" role="alert">{appState.error}</p>
    {/if}
  </section>
</div>
