// ========== Recherche globale d’entités (hors messagerie) ==========
// Disponible même si messagerie.js / mails.js ne sont pas chargés (desktop).

const MSG_MENTION_TOKEN_RE = /\{\{mn:([^:{}]+):([^:{}]*):([^}]+)\}\}/g;

function buildMsgMentionToken(type, payload, label) {
    return '{{mn:' + type + ':' + payload + ':' + encodeURIComponent(label || '') + '}}';
}

async function msgSearchMentionEntities(query, maxResults) {
    const q = (query || '').toLowerCase().trim();
    if (q.length < 1) return [];
    const results = [];
    const seen = {};

    function push(key, item) {
        if (seen[key]) return;
        seen[key] = true;
        results.push(item);
    }

    try {
        if (typeof ensureCrmDataLoaded === 'function') await ensureCrmDataLoaded();
    } catch (e) { /* ignore */ }
    try {
        if (typeof loadWorkProjectsData === 'function') await loadWorkProjectsData();
    } catch (e) { /* ignore */ }
    try {
        if (typeof loadMyBureauData === 'function') await loadMyBureauData();
    } catch (e) { /* ignore */ }

    const crm = typeof crmData !== 'undefined' ? crmData : null;
    if (crm && typeof hasCrmListAccess === 'function' && typeof hasCrmAccess === 'function' && hasCrmAccess()) {
        const prospectItems = typeof getAllCrmProspectsUnique === 'function'
            ? getAllCrmProspectsUnique()
            : (crm.lists || []).filter(hasCrmListAccess).flatMap(function(list) {
                return (list.prospects || []).map(function(p) { return { prospect: p, list: list }; });
            });
        const pushProspect = function(p, list) {
            if (typeof findCrmProspect === 'function' && !findCrmProspect(p.id)) return;
            if (typeof ensureProspectShape === 'function') ensureProspectShape(p);
            const structure = typeof getCrmProspectStructure === 'function' ? getCrmProspectStructure(p) : null;
            const personName = typeof getCrmProspectPersonName === 'function'
                ? getCrmProspectPersonName(p)
                : ((p.contactPrenom || '') + ' ' + (p.contactNom || '')).trim();
            const orgName = structure ? (structure.organisme || '') : (p.organisme || '');
            const hay = [
                personName,
                typeof getCrmProspectDisplayName === 'function' ? getCrmProspectDisplayName(p) : '',
                p.contactNom, p.contactPrenom, p.email, p.telFixe, p.telMobile, p.tel, p.ville,
                orgName,
                structure ? structure.ville : '',
                structure ? structure.cp : ''
            ].join(' ').toLowerCase();
            if (hay.indexOf(q) === -1) return;
            const label = typeof getCrmProspectMentionLabel === 'function'
                ? getCrmProspectMentionLabel(p)
                : (personName || orgName || 'Prospect');
            const subtitle = orgName && personName
                ? ('Prospect · ' + orgName)
                : ('Prospect · ' + (list && list.name ? list.name : 'CRM'));
            push('crm_p_' + p.id, {
                icon: '👤', title: label, subtitle: subtitle,
                token: buildMsgMentionToken('prospect', p.id, label)
            });
        };
        if (typeof getAllCrmProspectsUnique === 'function') {
            prospectItems.forEach(function(p) {
                const found = typeof findCrmProspect === 'function' ? findCrmProspect(p.id) : null;
                pushProspect(p, found ? found.list : null);
            });
        } else {
            prospectItems.forEach(function(item) { pushProspect(item.prospect, item.list); });
        }
        (crm.structures || []).forEach(function(s) {
            const hay = [s.organisme, s.ville, s.typeStructure, s.email].join(' ').toLowerCase();
            if (hay.indexOf(q) === -1) return;
            const label = s.organisme || 'Structure';
            push('crm_s_' + s.id, {
                icon: '🏛️', title: label, subtitle: 'Structure · CRM',
                token: buildMsgMentionToken('structure', s.id, label)
            });
        });
        (crm.deals || []).forEach(function(d) {
            const hay = [d.title, d.notes, d.description].join(' ').toLowerCase();
            if (hay.indexOf(q) !== -1) {
                const label = d.title || 'Dossier';
                push('crm_d_' + d.id, {
                    icon: '📁', title: label, subtitle: 'Dossier · CRM',
                    token: buildMsgMentionToken('deal', d.id, label)
                });
            }
            (d.internalNotes || []).forEach(function(n) {
                const hayN = [n.title, n.content, n.text].join(' ').toLowerCase();
                if (hayN.indexOf(q) === -1) return;
                const label = n.title || stripHtmlTags(n.content || n.text || '').slice(0, 60) || 'Note';
                const payload = n.activityId
                    ? d.id + '~' + n.activityId + '~' + n.id
                    : d.id + '~' + n.id;
                const type = n.activityId ? 'note_activity' : 'note_crm';
                push('crm_n_' + d.id + '_' + n.id, {
                    icon: '📝', title: label, subtitle: 'Note · ' + (d.title || 'Dossier'),
                    token: buildMsgMentionToken(type, payload, label)
                });
            });
            (d.documents || []).forEach(function(doc) {
                const hayD = [doc.name, doc.fileName, doc.titre].join(' ').toLowerCase();
                if (hayD.indexOf(q) === -1) return;
                const label = doc.name || doc.fileName || 'Document';
                push('crm_doc_' + d.id + '_' + doc.id, {
                    icon: '📎', title: label, subtitle: 'Document · ' + (d.title || 'Dossier'),
                    token: buildMsgMentionToken('doc_crm', d.id + '~' + doc.id, label)
                });
            });
            msgPushTaskDocumentMentions(d.tasks, 'crm_deal', 'crm_' + d.id, d.title || 'Dossier', push, q);
        });
    }

    if (typeof hasWorkProjectsAccess === 'function' && hasWorkProjectsAccess() && typeof workProjectsData !== 'undefined') {
        (workProjectsData.projects || []).filter(function(p) { return !p.archived; }).forEach(function(p) {
            const hay = [p.title, p.description].join(' ').toLowerCase();
            if (hay.indexOf(q) !== -1) {
                push('wp_' + p.id, {
                    icon: p.icon || '📁', title: p.title || 'Espace', subtitle: 'Espace de travail',
                    token: buildMsgMentionToken('project', p.id, p.title || 'Espace')
                });
            }
            (p.tasks || []).forEach(function(t) {
                if ((t.title || '').toLowerCase().indexOf(q) !== -1) {
                    const label = t.title || 'Tâche';
                    push('wp_t_' + p.id + '_' + t.id, {
                        icon: '✅', title: label, subtitle: 'Tâche · ' + (p.title || 'Espace'),
                        token: buildMsgMentionToken('task', 'work_project~' + p.id + '~' + t.id, label)
                    });
                }
                msgPushTaskDocumentMentions([t], 'work_project', p.id, p.title || 'Projet', push, q);
            });
        });
    }

    if (Array.isArray(window.projects)) {
        window.projects.forEach(function(p) {
            const isSpectacle = !!p.parentId || p.type === 'spectacle';
            const type = isSpectacle ? 'spectacle' : 'tournee';
            const icon = isSpectacle ? '🎪' : '🎭';
            const typeLabel = isSpectacle ? 'Spectacle' : 'Tournée';
            const label = p.name || typeLabel;
            const hayProj = [p.name, p.lieu, p.location, p.city, p.description].join(' ').toLowerCase();
            if (hayProj.indexOf(q) !== -1) {
                push('evt_' + p.id, {
                    icon: icon, title: label, subtitle: typeLabel + ' · Production',
                    token: buildMsgMentionToken(type, p.id, label)
                });
            }
            (p.documents || []).forEach(function(doc) {
                const hayD = [doc.name, doc.titre, doc.fileName, doc.type].join(' ').toLowerCase();
                if (hayD.indexOf(q) === -1) return;
                const docLabel = doc.name || doc.titre || doc.fileName || 'Document';
                push('evt_doc_' + p.id + '_' + doc.id, {
                    icon: '📎', title: docLabel, subtitle: 'Document · ' + label,
                    token: buildMsgMentionToken('doc_evt', p.id + '~' + doc.id, docLabel)
                });
            });
            (p.tasks || []).forEach(function(t) {
                if ((t.title || '').toLowerCase().indexOf(q) !== -1) {
                    const tLabel = t.title || 'Tâche';
                    push('evt_t_' + p.id + '_' + t.id, {
                        icon: '✅', title: tLabel, subtitle: 'Tâche · ' + label,
                        token: buildMsgMentionToken('task', 'event~' + p.id + '~' + t.id, tLabel)
                    });
                }
                msgPushTaskDocumentMentions([t], 'event', p.id, label, push, q);
            });
        });
    }

    const bureau = window.myBureau || { tasks: [] };
    (bureau.tasks || []).forEach(function(t) {
        if ((t.title || '').toLowerCase().indexOf(q) !== -1) {
            const label = t.title || 'Tâche';
            push('bureau_t_' + t.id, {
                icon: '📌', title: label, subtitle: 'Tâche · Mon Bureau',
                token: buildMsgMentionToken('task', 'personal~~' + t.id, label)
            });
        }
        msgPushTaskDocumentMentions([t], 'personal', '', 'Mon Bureau', push, q);
    });

    if (Array.isArray(window.workspaceElements)) {
        const typeLabels = { page: 'Note', idea: 'Idée', mindmap: 'Carte mentale', drawing: 'Dessin', file: 'Fichier', quicknote: 'Note rapide' };
        const icons = { page: '📝', idea: '💡', mindmap: '🧠', drawing: '🎨', file: '📎', quicknote: '📋' };
        window.workspaceElements.forEach(function(el) {
            const hay = [el.title, el.file_name, typeLabels[el.type]].join(' ').toLowerCase();
            if (hay.indexOf(q) === -1) return;
            const elLabel = el.title || el.file_name || 'Élément';
            if (el.type === 'file') {
                push('ws_doc_' + el.id, {
                    icon: '📎', title: elLabel, subtitle: 'Document · Mon Bureau',
                    token: buildMsgMentionToken('doc_bureau', el.id, elLabel)
                });
            } else {
                push('ws_' + el.id, {
                    icon: icons[el.type] || '📄', title: elLabel, subtitle: (typeLabels[el.type] || el.type) + ' · Mon Bureau',
                    token: buildMsgMentionToken('note_bureau', el.id, elLabel)
                });
            }
        });
    }

    await msgSearchWpWorkspaceDocs(q, push);

    if (typeof comptaJustificatifs !== 'undefined' && Array.isArray(comptaJustificatifs)) {
        const uid = String((typeof currentUser !== 'undefined' && currentUser && currentUser.id) || '');
        const canValidate = typeof canValidateFactures === 'function' && canValidateFactures();
        comptaJustificatifs.forEach(function(j) {
            if (typeof isDepotFacture === 'function' && !isDepotFacture(j)) return;
            if (!canValidate && String(j.deposeParUserId || '') !== uid) return;
            const hay = [j.libelle, j.fournisseur, j.numeroFacture, j.nomFichier, j.commentaire].join(' ').toLowerCase();
            if (hay.indexOf(q) === -1) return;
            const title = j.libelle || j.fournisseur || j.nomFichier || 'Facture';
            push('facture_' + j.id, {
                icon: '🧾', title: title, subtitle: 'Document · Comptabilité · Facture',
                token: buildMsgMentionToken('doc_compta', String(j.id), title)
            });
        });
    }

    if (typeof catalogueData !== 'undefined' && Array.isArray(catalogueData)) {
        catalogueData.forEach(function(f) {
            const hayF = [f.name, f.genre, f.duree, f.synopsis, f.resume, f.notes].join(' ').toLowerCase();
            if (hayF.indexOf(q) !== -1) {
                push('cat_' + f.id, {
                    icon: '📚', title: f.name || 'Fiche catalogue', subtitle: 'Catalogue · Production',
                    token: buildMsgMentionToken('catalogue_fiche', f.id, f.name || 'Fiche catalogue')
                });
            }
            (f.documents || []).forEach(function(doc) {
                const hay = [doc.name, doc.titre, doc.fileName, f.name].join(' ').toLowerCase();
                if (hay.indexOf(q) === -1) return;
                const docLabel = doc.name || doc.titre || doc.fileName || 'Document';
                push('cat_doc_' + f.id + '_' + doc.id, {
                    icon: '📎', title: docLabel, subtitle: 'Document · Catalogue · ' + (f.name || ''),
                    token: buildMsgMentionToken('doc_catalogue', f.id + '~' + doc.id, docLabel)
                });
            });
        });
    }

    if (typeof notesfrais !== 'undefined' && Array.isArray(notesfrais)) {
        const uid = typeof getNdfCurrentUserId === 'function' ? getNdfCurrentUserId() : String((currentUser && currentUser.id) || '');
        const isValidator = typeof hasNdfValidationPermission === 'function' && hasNdfValidationPermission();
        notesfrais.forEach(function(n) {
            if (typeof ndfNoteBelongsToUser === 'function' && !isValidator && !ndfNoteBelongsToUser(n, uid)) return;
            let hay = [n.titre, n.utilisateurNom, n.associationLabel, n.associationTexte].join(' ');
            (n.lignes || []).forEach(function(l) {
                hay += ' ' + (l.description || '') + ' ' + (l.type || '') + ' ' + (l.typeAutre || '');
            });
            if (hay.toLowerCase().indexOf(q) === -1) return;
            const statutLabel = typeof getNdfStatutLabel === 'function' ? getNdfStatutLabel(n.statut) : (n.statut || '');
            push('ndf_' + n.id, {
                icon: '💳', title: n.titre || 'Note de frais', subtitle: 'Comptabilité · Note de frais · ' + statutLabel,
                token: buildMsgMentionToken('ndf', String(n.id), n.titre || 'Note de frais')
            });
        });
    }

    const limit = maxResults || 15;
    return results.slice(0, limit);
}

function msgPushTaskDocumentMentions(tasks, source, projectId, parentLabel, push, q) {
    (tasks || []).forEach(function(t) {
        (t.documents || []).forEach(function(doc) {
            const hay = [doc.name, doc.fileName, t.title, parentLabel].join(' ').toLowerCase();
            if (hay.indexOf(q) === -1) return;
            const label = doc.name || doc.fileName || 'Document';
            push('tdoc_' + source + '_' + projectId + '_' + t.id + '_' + doc.id, {
                icon: '📎', title: label, subtitle: 'Document · ' + (t.title || parentLabel),
                token: buildMsgMentionToken('doc_task', source + '~' + (projectId || '') + '~' + t.id + '~' + doc.id, label)
            });
        });
    });
}

async function msgSearchWpWorkspaceDocs(q, push) {
    if (typeof hasWorkProjectsAccess !== 'function' || !hasWorkProjectsAccess()) return;
    if (typeof workProjectsData === 'undefined' || !workProjectsData.projects) return;
    const typeLabels = { page: 'Note', idea: 'Idée', mindmap: 'Carte mentale', drawing: 'Dessin', file: 'Fichier', quicknote: 'Note rapide' };
    const icons = { page: '📝', idea: '💡', mindmap: '🧠', drawing: '🎨', file: '📎', quicknote: '📋' };
    const projects = (workProjectsData.projects || []).filter(function(p) { return !p.archived; }).slice(0, 40);
    await Promise.all(projects.map(async function(p) {
        try {
            const res = await apiCall('workspace.php?visibility=team&project_id=' + encodeURIComponent(p.id), 'GET');
            const spaceLabel = p.title || 'Espace';
            (res && res.elements ? res.elements : []).forEach(function(el) {
                const hay = [el.title, el.file_name, typeLabels[el.type], spaceLabel].join(' ').toLowerCase();
                if (hay.indexOf(q) === -1) return;
                const label = el.title || el.file_name || (typeLabels[el.type] || 'Document');
                if (el.type === 'file') {
                    push('wp_doc_' + p.id + '_' + el.id, {
                        icon: '📎', title: label, subtitle: 'Document · Espace · ' + spaceLabel,
                        token: buildMsgMentionToken('doc_wp', p.id + '~' + el.id, label)
                    });
                } else {
                    push('wp_el_' + p.id + '_' + el.id, {
                        icon: icons[el.type] || '📄', title: label,
                        subtitle: (typeLabels[el.type] || el.type || 'Élément') + ' · Espace · ' + spaceLabel,
                        token: buildMsgMentionToken('doc_wp', p.id + '~' + el.id, label)
                    });
                }
            });
        } catch (e) { /* ignore */ }
    }));
}

async function msgFindTaskDocument(source, projectId, taskId, docId) {
    let task = null;
    if (source === 'crm_deal') {
        if (typeof ensureCrmDataLoaded === 'function') await ensureCrmDataLoaded();
        const dealId = String(projectId || '').replace(/^crm_/, '');
        const deal = typeof getCrmDealById === 'function' ? getCrmDealById(dealId) : null;
        task = (deal && deal.tasks || []).find(function(t) { return String(t.id) === String(taskId); });
    } else if (source === 'work_project') {
        if (typeof loadWorkProjectsData === 'function') await loadWorkProjectsData();
        const p = typeof getWorkProjectById === 'function' ? getWorkProjectById(projectId) : null;
        task = (p && p.tasks || []).find(function(t) { return String(t.id) === String(taskId); });
    } else if (source === 'personal') {
        if (typeof loadMyBureauData === 'function') await loadMyBureauData();
        task = ((window.myBureau && window.myBureau.tasks) || []).find(function(t) { return String(t.id) === String(taskId); });
    } else if (source === 'event') {
        const p = typeof getProjectById === 'function' ? getProjectById(projectId) : (window.projects || []).find(function(x) { return String(x.id) === String(projectId); });
        task = (p && p.tasks || []).find(function(t) { return String(t.id) === String(taskId); });
    }
    if (!task || !task.documents) return null;
    return task.documents.find(function(d) { return String(d.id) === String(docId); }) || null;
}

function msgOpenDocumentUrl(url) {
    if (!url) return false;
    window.open(url, '_blank', 'noopener');
    return true;
}

function stripHtmlTags(html) {
    if (!html) return '';
    const d = document.createElement('div');
    d.innerHTML = html;
    return (d.textContent || d.innerText || '').trim();
}

async function openMsgEntityMention(type, payload) {
    if (!type || payload == null) return;
    try {
        if (typeof ensureCrmDataLoaded === 'function') await ensureCrmDataLoaded();
    } catch (e) { /* ignore */ }

    const openCrm = function(fn) {
        if (typeof openCrmPage === 'function') openCrmPage();
        setTimeout(fn, 200);
    };

    switch (type) {
        case 'prospect':
            openCrm(function() { if (typeof openCrmProspectFiche === 'function') openCrmProspectFiche(payload); });
            break;
        case 'deal':
            openCrm(function() { if (typeof openCrmDealFiche === 'function') openCrmDealFiche(payload); });
            break;
        case 'structure':
            openCrm(function() { if (typeof openCrmStructureFiche === 'function') openCrmStructureFiche(payload); });
            break;
        case 'project':
            if (typeof openWorkProjectsListPage === 'function') openWorkProjectsListPage();
            setTimeout(function() {
                if (typeof openWorkProjectPage === 'function') openWorkProjectPage(payload);
            }, 200);
            break;
        case 'task': {
            const parts = String(payload).split('~');
            const source = parts[0] || '';
            const projectId = parts[1] || '';
            const taskId = parts[2] || '';
            if (source === 'event') {
                const p = typeof getProjectById === 'function' ? getProjectById(projectId) : null;
                const isSpectacle = p && (!!p.parentId || p.type === 'spectacle');
                if (typeof closeAllPages === 'function') closeAllPages();
                if (isSpectacle && typeof viewSpectacle === 'function') viewSpectacle(projectId);
                else if (typeof viewTournee === 'function') viewTournee(projectId);
                setTimeout(function() {
                    if (typeof openTaskDetailModal === 'function') openTaskDetailModal(projectId, taskId);
                }, 300);
            } else if (typeof openTaskFiche === 'function') {
                openTaskFiche(source, projectId, taskId);
            }
            break;
        }
        case 'note_crm': {
            const parts = String(payload).split('~');
            const dealId = parts[0];
            const noteId = parts[1];
            openCrm(function() {
                if (typeof openCrmDealFiche === 'function') openCrmDealFiche(dealId);
                setTimeout(function() {
                    if (typeof openCrmDealNotesPage === 'function') openCrmDealNotesPage(noteId);
                }, 250);
            });
            break;
        }
        case 'note_activity': {
            const parts = String(payload).split('~');
            const dealId = parts[0];
            const activityId = parts[1];
            const noteId = parts[2];
            openCrm(function() {
                if (typeof openCrmDealFiche === 'function') openCrmDealFiche(dealId);
                setTimeout(function() {
                    if (typeof openCrmDealActivityNoteModal === 'function') openCrmDealActivityNoteModal(activityId, noteId);
                }, 300);
            });
            break;
        }
        case 'note_bureau':
            await msgOpenBureauWorkspaceMention(payload, 'note');
            break;
        case 'doc_bureau':
            await msgOpenBureauWorkspaceMention(payload, 'file');
            break;
        case 'tournee':
            if (typeof openEvenementielHomePage === 'function') openEvenementielHomePage();
            setTimeout(function() { if (typeof viewTournee === 'function') viewTournee(payload); }, 200);
            break;
        case 'spectacle':
            if (typeof openEvenementielHomePage === 'function') openEvenementielHomePage();
            setTimeout(function() { if (typeof viewSpectacle === 'function') viewSpectacle(payload); }, 200);
            break;
        case 'doc_crm': {
            const parts = String(payload).split('~');
            if (typeof openCrmDealDocumentMention === 'function') {
                openCrmDealDocumentMention(parts[0], parts[1]);
            } else {
                openCrm(function() {
                    if (typeof openCrmDealFiche === 'function') openCrmDealFiche(parts[0]);
                    setTimeout(function() {
                        if (typeof openCrmDealDocumentsPage === 'function') openCrmDealDocumentsPage();
                    }, 280);
                });
            }
            break;
        }
        case 'doc_evt': {
            const parts = String(payload).split('~');
            const projectId = parts[0];
            const docId = parts[1];
            const p = typeof getProjectById === 'function' ? getProjectById(projectId) : (window.projects || []).find(function(x) { return String(x.id) === String(projectId); });
            const doc = p && (p.documents || []).find(function(d) { return String(d.id) === String(docId); });
            if (doc && msgOpenDocumentUrl(doc.downloadUrl || doc.url)) break;
            if (typeof closeAllPages === 'function') closeAllPages();
            const isSpectacle = p && (!!p.parentId || p.type === 'spectacle');
            if (isSpectacle && typeof openDocumentsPage === 'function') {
                openDocumentsPage(projectId);
            } else if (typeof viewTournee === 'function') {
                viewTournee(projectId);
                setTimeout(function() {
                    if (typeof openTourneeDocumentsPage === 'function') openTourneeDocumentsPage();
                }, 300);
            }
            break;
        }
        case 'doc_task': {
            const parts = String(payload).split('~');
            const doc = await msgFindTaskDocument(parts[0], parts[1], parts[2], parts[3]);
            if (doc && msgOpenDocumentUrl(doc.downloadUrl || doc.url)) break;
            if (typeof openTaskFiche === 'function') openTaskFiche(parts[0], parts[1], parts[2]);
            break;
        }
        case 'doc_wp': {
            const parts = String(payload).split('~');
            const projectId = parts[0];
            const fileId = parts[1];
            if (typeof openWorkProjectsListPage === 'function') openWorkProjectsListPage();
            setTimeout(async function() {
                if (typeof openWorkProjectPage === 'function') openWorkProjectPage(projectId, { tab: 'documents' });
                setTimeout(function() {
                    if (typeof openWpWorkspaceFile === 'function') openWpWorkspaceFile(fileId);
                    else if (typeof openWorkspaceElement === 'function') openWorkspaceElement(fileId);
                    else if (typeof getWorkspaceFileUrl === 'function') msgOpenDocumentUrl(getWorkspaceFileUrl(fileId));
                }, 350);
            }, 200);
            break;
        }
        case 'doc_compta':
            if (typeof openComptabiliteHomePage === 'function') openComptabiliteHomePage();
            setTimeout(function() {
                if (typeof openDepotFactureDetailModal === 'function') openDepotFactureDetailModal(payload);
            }, 200);
            break;
        case 'doc_catalogue': {
            const parts = String(payload).split('~');
            const ficheId = parts[0];
            const docId = parts[1];
            let doc = null;
            if (typeof catalogueData !== 'undefined' && Array.isArray(catalogueData)) {
                const fiche = catalogueData.find(function(f) { return String(f.id) === String(ficheId); });
                doc = fiche && (fiche.documents || []).find(function(d) { return String(d.id) === String(docId); });
            }
            if (doc && msgOpenDocumentUrl(doc.url || doc.downloadUrl)) break;
            if (typeof openEvenementielHomePage === 'function') openEvenementielHomePage();
            setTimeout(function() {
                if (typeof viewCatalogueFiche === 'function') viewCatalogueFiche(ficheId);
            }, 200);
            break;
        }
        case 'catalogue_fiche':
            if (typeof openEvenementielHomePage === 'function') openEvenementielHomePage();
            setTimeout(function() {
                if (typeof loadCatalogueData === 'function') {
                    loadCatalogueData().then(function() {
                        if (typeof viewCatalogueFiche === 'function') viewCatalogueFiche(payload);
                    });
                } else if (typeof viewCatalogueFiche === 'function') {
                    viewCatalogueFiche(payload);
                }
            }, 200);
            break;
        case 'ndf': {
            const isValidator = typeof hasNdfValidationPermission === 'function' && hasNdfValidationPermission();
            const ctx = isValidator ? 'validation' : 'user';
            if (typeof openComptabiliteHomePage === 'function') openComptabiliteHomePage();
            setTimeout(function() {
                if (typeof openNdfDetailModal === 'function') openNdfDetailModal(payload, ctx);
            }, 200);
            break;
        }
        default:
            break;
    }
}
window.openMsgEntityMention = openMsgEntityMention;

let msgWsAccessPending = null;

const MSG_WS_TYPE_LABELS = {
    page: 'Note', idea: 'Idée', mindmap: 'Carte mentale', drawing: 'Dessin',
    file: 'Fichier', quicknote: 'Note rapide'
};

async function msgOpenBureauWorkspaceMention(elementId, mentionKind) {
    if (!elementId) return;
    try {
        const res = await apiCall('workspace.php?id=' + encodeURIComponent(elementId), 'GET');
        if (res?.success && res.element) {
            if (res.element.access_role === 'viewer' && typeof openWorkspaceElementConsultation === 'function') {
                openWorkspaceElementConsultation(res.element);
            } else {
                await msgOpenWorkspaceElementFromMention(res.element, mentionKind);
            }
            return;
        }
    } catch (e) {
        if (e.message === 'Accès refusé') {
            await msgShowWorkspaceAccessModal(elementId, mentionKind);
            return;
        }
        if (typeof showToast === 'function') showToast('❌ ' + e.message, 'error');
    }
}

async function msgShowWorkspaceAccessModal(elementId, mentionKind) {
    try {
        const previewRes = await apiCall('workspace.php?action=mention_preview&id=' + encodeURIComponent(elementId), 'GET');
        if (!previewRes?.success || !previewRes.preview) {
            if (typeof showToast === 'function') showToast('❌ Partage non disponible', 'error');
            return;
        }
        msgWsAccessPending = { elementId: elementId, mentionKind: mentionKind, preview: previewRes.preview };
        const modal = document.getElementById('msgWorkspaceAccessModal');
        const titleEl = document.getElementById('msgWsAccessTitle');
        const metaEl = document.getElementById('msgWsAccessMeta');
        const typeEl = document.getElementById('msgWsAccessType');
        if (titleEl) titleEl.textContent = previewRes.preview.title || 'Élément partagé';
        if (typeEl) typeEl.textContent = MSG_WS_TYPE_LABELS[previewRes.preview.type] || previewRes.preview.type || 'Document';
        if (metaEl) {
            const owner = previewRes.preview.created_by_name || 'Un collègue';
            metaEl.textContent = 'Partagé par ' + owner + ' via la messagerie';
        }
        if (modal) modal.classList.add('active');
    } catch (e) {
        if (typeof showToast === 'function') showToast('❌ ' + (e.message || 'Impossible d\'accéder à cet élément'), 'error');
    }
}

function msgCloseWorkspaceAccessModal() {
    msgWsAccessPending = null;
    const modal = document.getElementById('msgWorkspaceAccessModal');
    if (modal) modal.classList.remove('active');
}

async function msgGrantWorkspaceAccess(mode) {
    if (!msgWsAccessPending) return;
    const pending = msgWsAccessPending;
    msgCloseWorkspaceAccessModal();
    if (mode === 'view') {
        try {
            const res = await apiCall('workspace.php?action=mention_view&id=' + encodeURIComponent(pending.elementId), 'GET');
            if (res?.success && res.element) {
                if (typeof openWorkspaceElementConsultation === 'function') {
                    openWorkspaceElementConsultation(res.element);
                }
                if (typeof showToast === 'function') showToast('👁️ Ouvert en consultation seule', 'success');
            }
        } catch (e) {
            if (typeof showToast === 'function') showToast('❌ ' + e.message, 'error');
        }
        return;
    }
    try {
        const res = await apiCall('workspace.php', 'POST', {
            action: 'grant_access',
            element_id: pending.elementId,
            mode: 'collaborate'
        });
        if (res?.success && res.element) {
            await msgOpenWorkspaceElementFromMention(res.element, pending.mentionKind);
            if (typeof showToast === 'function') {
                showToast('✅ Ajouté à votre espace — les modifications seront synchronisées', 'success');
            }
        }
    } catch (e) {
        if (typeof showToast === 'function') showToast('❌ ' + e.message, 'error');
    }
}

async function msgOpenWorkspaceElementFromMention(el, mentionKind) {
    const isFile = el.type === 'file' || mentionKind === 'file';

    if (isFile) {
        const url = typeof getWorkspaceFileUrl === 'function' ? getWorkspaceFileUrl(el.id || el) : null;
        if (url) window.open(url, '_blank');
        return;
    }

    if (typeof openMyBureauPage === 'function') {
        await Promise.resolve(openMyBureauPage());
    }
    if (typeof switchBureauTab === 'function') switchBureauTab('workspace');
    if (typeof loadWorkspaceElements === 'function') await loadWorkspaceElements();
    setTimeout(function() {
        if (typeof openWorkspaceElement === 'function') {
            openWorkspaceElement(el.id || el);
        }
    }, 280);
}

function parseMsgMentionToken(token) {
    const m = /^\{\{mn:([^:{}]+):([^:{}]*):([^}]+)\}\}$/.exec(token || '');
    if (!m) return null;
    return { type: m[1], payload: m[2] };
}

async function globalSearchAppendAppEntities(query, pushResult) {
    const items = await msgSearchMentionEntities(query, 30);
    items.forEach(function(item, i) {
        const parsed = parseMsgMentionToken(item.token);
        if (!parsed) return;
        const key = 'gs_' + parsed.type + '_' + parsed.payload + '_' + i;
        const result = {
            icon: item.icon,
            title: item.title,
            subtitle: item.subtitle,
            action: function() { openMsgEntityMention(parsed.type, parsed.payload); }
        };
        if (parsed.type === 'project' && typeof getWorkProjectById === 'function') {
            const p = getWorkProjectById(parsed.payload);
            if (p && typeof getWpProjectImageUrl === 'function') {
                const imgUrl = getWpProjectImageUrl(p);
                if (imgUrl) result.imageUrl = imgUrl;
            }
        }
        pushResult(key, result);
    });
}
window.globalSearchAppendAppEntities = globalSearchAppendAppEntities;


window.buildMsgMentionToken = buildMsgMentionToken;
window.msgSearchMentionEntities = msgSearchMentionEntities;
window.msgSearchWpWorkspaceDocs = msgSearchWpWorkspaceDocs;
window.openMsgEntityMention = openMsgEntityMention;
window.globalSearchAppendAppEntities = globalSearchAppendAppEntities;
window.parseMsgMentionToken = parseMsgMentionToken;
window.msgOpenDocumentUrl = msgOpenDocumentUrl;
window.msgFindTaskDocument = msgFindTaskDocument;
window.stripHtmlTags = stripHtmlTags;
window.msgGrantWorkspaceAccess = msgGrantWorkspaceAccess;
window.msgCloseWorkspaceAccessModal = msgCloseWorkspaceAccessModal;
