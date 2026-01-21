// Vidi XP - Local Storage Helpers

const STORAGE_KEY_RECENT = 'vidi_recent_dashboards';
const MAX_RECENT = 10;

const VidiStorage = {
  // Get recent dashboards
  getRecent() {
    try {
      const data = localStorage.getItem(STORAGE_KEY_RECENT);
      return data ? JSON.parse(data) : [];
    } catch (e) {
      console.error('Failed to load recent dashboards:', e);
      return [];
    }
  },

  // Add a dashboard to recent list
  addRecent(dashboard) {
    try {
      let recent = this.getRecent();

      // Remove if already exists
      recent = recent.filter(r => r.id !== dashboard.id);

      // Add to front
      recent.unshift({
        id: dashboard.id,
        xp_name: dashboard.xp_name,
        timestamp: Date.now()
      });

      // Trim to max size
      if (recent.length > MAX_RECENT) {
        recent = recent.slice(0, MAX_RECENT);
      }

      localStorage.setItem(STORAGE_KEY_RECENT, JSON.stringify(recent));
    } catch (e) {
      console.error('Failed to save recent dashboard:', e);
    }
  },

  // Remove a dashboard from recent list
  removeRecent(id) {
    try {
      let recent = this.getRecent();
      recent = recent.filter(r => r.id !== id);
      localStorage.setItem(STORAGE_KEY_RECENT, JSON.stringify(recent));
    } catch (e) {
      console.error('Failed to remove recent dashboard:', e);
    }
  },

  // Clear all recent dashboards
  clearRecent() {
    try {
      localStorage.removeItem(STORAGE_KEY_RECENT);
    } catch (e) {
      console.error('Failed to clear recent dashboards:', e);
    }
  }
};
