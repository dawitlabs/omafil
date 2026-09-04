<script lang="ts">
  import AddIcon from '@fluentui/svg-icons/icons/add_20_regular.svg?no-inline'
  import ArrowLeftIcon from '@fluentui/svg-icons/icons/arrow_left_20_regular.svg?no-inline'
  import ArrowRightIcon from '@fluentui/svg-icons/icons/arrow_right_20_regular.svg?no-inline'
  import ArrowUpIcon from '@fluentui/svg-icons/icons/arrow_up_20_regular.svg?no-inline'
  import ChevronRightIcon from '@fluentui/svg-icons/icons/chevron_right_20_regular.svg?no-inline'
  import DismissIcon from '@fluentui/svg-icons/icons/dismiss_20_regular.svg?no-inline'
  import FolderIcon from '@fluentui/svg-icons/icons/folder_20_regular.svg?no-inline'
  import HomeIcon from '@fluentui/svg-icons/icons/home_20_regular.svg?no-inline'
  import MicIcon from '@fluentui/svg-icons/icons/mic_20_regular.svg?no-inline'
  import MinimizeIcon from '@fluentui/svg-icons/icons/subtract_20_regular.svg?no-inline'
  import SearchIcon from '@fluentui/svg-icons/icons/search_20_regular.svg?no-inline'
  import SettingsIcon from '@fluentui/svg-icons/icons/settings_20_regular.svg?no-inline'
  import MaximizeIcon from '@fluentui/svg-icons/icons/square_20_regular.svg?no-inline'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { navigation } from '../../navigation.svelte'

  const appWindow = getCurrentWindow()

  async function minimizeWindow() {
    await appWindow.minimize()
  }

  async function toggleMaximize() {
    if (await appWindow.isMaximized()) {
      await appWindow.unmaximize()
      return
    }

    await appWindow.maximize()
  }

  async function closeWindow() {
    await appWindow.close()
  }
</script>

<header class="app-header">
  <div class="app-header__tabbar">
    <div class="app-header__tab app-header__tab--active" aria-current="page">
      <span class="masked-icon app-header__tab-icon" style="--icon: url({HomeIcon})" aria-hidden="true"></span>
      <span>{navigation.label}</span>
      <span class="masked-icon app-header__tab-close" style="--icon: url({DismissIcon})" aria-hidden="true"></span>
    </div>

    <button class="app-header__new-tab" type="button" aria-label="New tab" title="Tabs are not available yet" disabled>
      <span class="masked-icon app-header__icon" style="--icon: url({AddIcon})" aria-hidden="true"></span>
    </button>

    <div class="app-header__drag-space" data-tauri-drag-region></div>

    <button class="app-header__settings-button" type="button" aria-label="Settings" title="Settings are not available yet" disabled>
      <span class="masked-icon app-header__icon" style="--icon: url({SettingsIcon})" aria-hidden="true"></span>
    </button>

    <div class="app-header__window-controls" aria-label="Window controls">
      <button class="app-header__window-button" type="button" aria-label="Minimize" onclick={minimizeWindow}>
        <span class="masked-icon app-header__icon" style="--icon: url({MinimizeIcon})" aria-hidden="true"></span>
      </button>
      <button class="app-header__window-button" type="button" aria-label="Maximize" onclick={toggleMaximize}>
        <span class="masked-icon app-header__icon" style="--icon: url({MaximizeIcon})" aria-hidden="true"></span>
      </button>
      <button class="app-header__window-button app-header__window-button--close" type="button" aria-label="Close" onclick={closeWindow}>
        <span class="masked-icon app-header__icon" style="--icon: url({DismissIcon})" aria-hidden="true"></span>
      </button>
    </div>
  </div>

  <div class="app-header__commandbar">
    <div class="app-header__navigation" aria-label="File navigation controls">
      <button class="app-header__icon-button" type="button" aria-label="Back" onclick={() => navigation.back()} disabled={!navigation.canGoBack}>
        <span class="masked-icon app-header__icon" style="--icon: url({ArrowLeftIcon})" aria-hidden="true"></span>
      </button>
      <button class="app-header__icon-button" type="button" aria-label="Forward" onclick={() => navigation.forward()} disabled={!navigation.canGoForward}>
        <span class="masked-icon app-header__icon" style="--icon: url({ArrowRightIcon})" aria-hidden="true"></span>
      </button>
      <button class="app-header__icon-button" type="button" aria-label="Up" onclick={() => navigation.up()} disabled={!navigation.canGoUp}>
        <span class="masked-icon app-header__icon" style="--icon: url({ArrowUpIcon})" aria-hidden="true"></span>
      </button>
    </div>

    <nav class="app-header__path" aria-label="Breadcrumb">
      <button class="app-header__crumb" type="button" onclick={() => navigation.goHome()}>
        <span class="masked-icon app-header__tab-icon" style="--icon: url({navigation.view.kind === 'home' ? HomeIcon : FolderIcon})" aria-hidden="true"></span>
        <span>{navigation.view.kind === 'home' ? 'Home' : 'Files'}</span>
      </button>

      {#each navigation.crumbs as crumb (crumb.path)}
        <span class="masked-icon app-header__crumb-separator" style="--icon: url({ChevronRightIcon})" aria-hidden="true"></span>
        <button class="app-header__crumb" type="button" onclick={() => navigation.open(crumb.path)} aria-current={navigation.isCurrentPath(crumb.path) ? 'page' : undefined}>
          <span>{crumb.name}</span>
        </button>
      {/each}

      {#if navigation.view.kind === 'placeholder'}
        <span class="masked-icon app-header__crumb-separator" style="--icon: url({ChevronRightIcon})" aria-hidden="true"></span>
        <span class="app-header__crumb app-header__crumb--static">{navigation.label}</span>
      {/if}
    </nav>

    <label class="app-header__search">
      <span class="masked-icon app-header__icon" style="--icon: url({SearchIcon})" aria-hidden="true"></span>
      <input type="search" placeholder="Search" aria-label="Search" title="Search is not available yet" disabled />
      <span class="masked-icon app-header__icon" style="--icon: url({MicIcon})" aria-hidden="true"></span>
    </label>
  </div>
</header>
