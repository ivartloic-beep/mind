// ========== Gestion — société unique (sélecteur multi-société désactivé) ==========
// API conservée pour compatibilité (normalizeUserSocieties / setCurrentEntity / isLcom).
// L’identifiant interne « lcom » reste pour les payloads API existants ; ce n’est plus une marque UI.

var currentEntity = 'lcom';

function normalizeUserSocieties(societies) {
    return ['lcom'];
}

function setCurrentEntity(entity) {
    currentEntity = 'lcom';
}

/** Toujours vrai : instance mono-organisation. */
function isLcom() {
    return true;
}

function openProjectsListPageLcom() {
    setCurrentEntity('lcom');
    if (typeof openProjectsListPage === 'function') openProjectsListPage();
}

function openComptabiliteLcom() {
    setCurrentEntity('lcom');
    if (typeof openComptabiliteHomePage === 'function') openComptabiliteHomePage();
    else if (typeof openDepotJustificatifsPage === 'function') openDepotJustificatifsPage();
    else if (typeof openNotesFraisPage === 'function') openNotesFraisPage();
}

function openCrmPageLcom() {
    setCurrentEntity('lcom');
    if (typeof openCrmPage === 'function') openCrmPage();
}

/** Toutes les sections métier sont visibles (pas de filtrage par société). */
function applySocietiesVisibility() {
    var homeSpaceWorkProjects = document.getElementById('homeSpaceWorkProjects');
    if (homeSpaceWorkProjects) homeSpaceWorkProjects.style.display = 'none';
}

/** Sélecteur société désactivé — organisation unique configurable ailleurs. */
function renderSocietySelector(containerId, options) {
    setCurrentEntity('lcom');
    var container = document.getElementById(containerId);
    if (container) {
        container.innerHTML = '';
        container.style.display = 'none';
    }
    if (options && options.contentId) {
        var contentEl = document.getElementById(options.contentId);
        if (contentEl) contentEl.style.display = '';
    }
    if (options && typeof options.onChange === 'function') options.onChange('lcom');
}

if (typeof window !== 'undefined') {
    window.normalizeUserSocieties = normalizeUserSocieties;
    window.setCurrentEntity = setCurrentEntity;
    window.isLcom = isLcom;
    window.applySocietiesVisibility = applySocietiesVisibility;
    window.renderSocietySelector = renderSocietySelector;
}
