# Verifica dei requisiti

Matrice requisito → verifica per il change `photo-catalog-foundation`. Fonti: test Rust (`#[test]`), test UI (Vitest, `*.test.ts(x)`) e verifiche manuali registrate in `tasks.md`.

Convenzioni:

- Test Rust: `percorso::nome_funzione`. I test marcati **[model-tests]** girano solo con `--features model-tests` su Windows (modelli reali); quelli marcati **[heif]** dipendono dalla feature `heif`.
- Test UI: `percorso` › "nome del test".
- Verifiche manuali: "manuale: <cosa>, task N.N" (solo task spuntati contano come eseguiti).
- Stato: **coperto** (tutti gli scenari verificati), **parziale** (alcune clausole o scenari senza verifica), **manca** (nessuna verifica).

Riepilogo: 89 requisiti, **72 coperti**, **14 parziali**, **3 mancanti**.

## ai-classification

| Requisito | Verifica | Stato |
|---|---|---|
| Esecuzione esclusivamente locale | Nessun test. Evidenza solo strutturale (`pc-ai` non dipende da client HTTP; gazetteer offline in `crates/pc-ai/src/geo.rs`). Verifica manuale prevista dal task 14.2, non ancora eseguito | manca |
| Tassonomia configurabile | `crates/pc-ai/src/taxonomy.rs::edits_are_versioned_validated_and_persisted`; `crates/pc-ai/tests/models.rs::default_taxonomy_classifies_mountain_and_persists_embedding` [model-tests]; `crates/pc-ai/src/text_cache.rs::cache_avoids_loading_encoder_and_invalidates_on_version_or_model_change` | coperto |
| Classificazione zero-shot con confidenza | `crates/pc-ai/src/classify.rs::confidence_is_per_group_and_respects_thresholds`; `crates/pc-ai/tests/models.rs::clip_zero_shot_matches_scene` [model-tests]; `crates/pc-ai/tests/models.rs::default_taxonomy_classifies_mountain_and_persists_embedding` [model-tests] | coperto |
| Rilevamento e conteggio persone | `crates/pc-ai/tests/models.rs::persons_counted_in_group_and_absent_in_landscape` [model-tests]; `crates/pc-store/src/analysis.rs::reanalysis_preserves_manual_groups_and_tracks_current_versions` (persistenza conteggio/aree) | coperto |
| Geolocalizzazione | `crates/pc-ai/src/geo.rs::nearest_city_with_italian_name`; `crates/pc-ai/src/geo.rs::place_from_directory_names`; `crates/pc-ai/src/geo.rs::kd_tree_handles_antimeridian_and_matches_brute_force`; fonte mostrata: `app/ui/src/modules/library/LibraryPage.test.tsx` › "griglia, badge, selezione e ispettore" | coperto |
| Correzione manuale prevale | `crates/pc-store/src/analysis.rs::reanalysis_preserves_manual_groups_and_tracks_current_versions`; `crates/pc-store/src/repo.rs::manual_location_is_not_overwritten` | coperto |
| Versionamento delle analisi | `crates/pc-store/src/analysis.rs::reanalysis_preserves_manual_groups_and_tracks_current_versions` (`pending`/`stale_count` per model_ver/taxonomy_ver) | coperto |
| Analisi riprendibile | `crates/pc-jobs/tests/manager.rs::interrupted_job_resumes_only_pending_items`; `crates/pc-jobs/src/pipeline.rs::journal_skips_finished_items_on_resume`; `crates/pc-store/src/analysis.rs::reanalysis_preserves_manual_groups_and_tracks_current_versions` (file già analizzati esclusi) | coperto |

## bulk-operations

| Requisito | Verifica | Stato |
|---|---|---|
| Selezione multipla | `app/ui/src/modules/library/LibraryPage.test.tsx` › "clic, Ctrl e Shift"; `app/ui/src/modules/library/BulkBar.test.tsx` › "selects unseen filtered results, inverts and confirms destructive edits"; `app/ui/src/modules/library/BulkBar.test.tsx` › "rectangle selects rows independently of the virtual DOM" | coperto |
| Azioni bulk disponibili | Tag: `crates/pc-jobs/src/bulk.rs::five_thousand_items_one_undo_and_partial_failure`, `crates/pc-store/src/bulk.rs::undo_tag_on_200_files`; cataloghi e correzione campo: `crates/pc-store/src/bulk.rs::undo_catalog_and_labels`; export di una selezione: `crates/pc-organize/tests/operations.rs::csv_and_json_export_multivalues_accents_and_scope`. Nessun test per "assegna persona" e "rianalizza" in bulk | parziale |
| Esecuzione come job per selezioni grandi | `crates/pc-jobs/src/bulk.rs::five_thousand_items_one_undo_and_partial_failure` (5.000 elementi come job) | coperto |
| Conferma per azioni distruttive | `app/ui/src/modules/library/BulkBar.test.tsx` › "selects unseen filtered results, inverts and confirms destructive edits" (conferma con conteggio, nessun avvio se annullata). Nessun test che "rimuovi dalla libreria" lasci intatti i file originali | parziale |
| Annullamento delle modifiche ai metadati | `crates/pc-store/src/bulk.rs::undo_tag_on_200_files`; `crates/pc-store/src/bulk.rs::undo_only_reverts_actually_changed_items`; `crates/pc-store/src/bulk.rs::undo_catalog_and_labels`; `crates/pc-store/src/bulk.rs::undo_history_is_bounded_and_empty_undo_is_noop` | coperto |
| Esito delle azioni bulk | `crates/pc-jobs/src/bulk.rs::five_thousand_items_one_undo_and_partial_failure` (1 fallito su 5.001); `app/src-tauri/src/diagnostics.rs::retry_restricts_each_job_kind_to_failed_files` | coperto |

## catalog-export

| Requisito | Verifica | Stato |
|---|---|---|
| File strutturato di catalogazione | `crates/pc-organize/tests/operations.rs::csv_and_json_export_multivalues_accents_and_scope` (solo campi path/tags/people). Nessun test sull'insieme completo dei campi obbligatori | parziale |
| Formati di export | `crates/pc-organize/tests/operations.rs::csv_and_json_export_multivalues_accents_and_scope` (CSV con separatore e BOM, JSON con multi-valore); `app/ui/src/modules/organize/OrganizePage.test.tsx` › "esporta in CSV con separatore, BOM e campi scelti" | coperto |
| Ambito e campi selezionabili | `crates/pc-organize/tests/operations.rs::csv_and_json_export_multivalues_accents_and_scope` (ambito selezione, campi scelti); `app/ui/src/modules/organize/OrganizePage.test.tsx` › "esporta in CSV con separatore, BOM e campi scelti". Non verificati ambito catalogo/filtro e campo confidenza | parziale |
| Schema versionato | `crates/pc-organize/tests/operations.rs::csv_and_json_export_multivalues_accents_and_scope` (`schema_version` in CSV e JSON) | coperto |
| Export come job | `crates/pc-organize/tests/operations.rs::cancelled_export_leaves_no_partial_or_destination`; `crates/pc-organize/tests/operations.rs::csv_and_json_export_multivalues_accents_and_scope` (eseguito come job) | coperto |

## catalog-library

| Requisito | Verifica | Stato |
|---|---|---|
| Libreria unica dei media | `crates/pc-store/src/repo.rs::media_upsert_is_idempotent_by_path`; `crates/pc-store/tests/writer.rs::data_persists_after_reopen`; `crates/pc-ingest/tests/import.rs::full_import_incremental_rescan_moves_and_missing` | coperto |
| Modello di media estendibile | `crates/pc-store/src/migrate.rs::empty_db_migrates_to_latest`; `crates/pc-store/src/migrate.rs::backup_is_written_for_existing_older_db`; `crates/pc-store/src/migrate.rs::newer_schema_is_refused` | coperto |
| Cataloghi manuali | `crates/pc-store/src/repo.rs::tags_and_catalogs` (creazione, rinomina, più cataloghi, eliminazione senza perdita di file) | coperto |
| Cataloghi smart | `crates/pc-store/tests/query.rs::smart_catalog_matches_rules_and_updates_automatically`; `crates/pc-store/tests/query.rs::malicious_rules_are_rejected_and_tables_survive`; `crates/pc-store/src/rules.rs::parses_and_compiles_nested_rules`; `crates/pc-store/src/rules.rs::rejects_unknown_fields_and_injection_attempts`; `crates/pc-store/src/rules.rs::depth_is_limited`; `app/ui/src/modules/catalogs/CatalogsPage.test.tsx` › "edits nested AND/OR rules, previews the count and saves a smart catalog" | coperto |
| Tag utente | `crates/pc-store/src/repo.rs::tags_and_catalogs` (assegna/rimuovi). Nessun test che una rianalisi lasci invariati i tag utente | parziale |
| Ricerca e filtri | `crates/pc-store/tests/query.rs::combined_filters_and_keyset_pagination_on_100k` (< 1 s su 100.000 righe); `crates/pc-store/src/repo.rs::fts_finds_by_place_and_tag`; `crates/pc-store/src/rules.rs::fts_query_quotes_terms`; `app/ui/src/modules/library/LibraryPage.test.tsx` › "faccette e chip modificano il filtro della query" | coperto |
| Stato dei file sul disco | `crates/pc-ingest/tests/import.rs::full_import_incremental_rescan_moves_and_missing` (stato "mancante" senza rimozione); `crates/pc-store/src/repo.rs::duplicates_and_relink`; badge "Mancante": `app/ui/src/modules/library/LibraryPage.test.tsx` › "griglia, badge, selezione e ispettore" | coperto |

## desktop-shell

| Requisito | Verifica | Stato |
|---|---|---|
| Layout a pannelli | `app/ui/src/components/shell/shell.test.tsx` › "lo stato dei pannelli è persistito e ripristinato"; `app/ui/src/components/shell/shell.test.tsx` › "il pannello sinistro si ridimensiona da tastiera entro i limiti" | coperto |
| Moduli di navigazione | Cambio modulo indiretto in `app/ui/src/components/shell/shell.test.tsx` › "pill del job e chip RAM seguono gli eventi" (→ Job) e › "la barra di stato conta gli errori e apre la diagnostica" (→ Diagnostica). Nessun test che tutti i 9 moduli siano registrati | parziale |
| Griglia di miniature performante | `app/ui/src/modules/library/LibraryPage.test.tsx` › "ordinamento e dimensione miniature"; `app/ui/src/modules/library/BulkBar.test.tsx` › "rectangle selects rows independently of the virtual DOM" (100.000 id); manuale: scorrimento su 100.000 elementi sintetici, task 7.11 | coperto |
| Pannello di ispezione | `app/ui/src/modules/library/LibraryPage.test.tsx` › "griglia, badge, selezione e ispettore" (visualizzazione dei campi); `crates/pc-store/src/detail.rs::detail_and_summaries`. Non verificata la modifica di catalogazioni, tag e cataloghi dal pannello | parziale |
| Indicatori globali | `app/ui/src/components/shell/shell.test.tsx` › "pill del job e chip RAM seguono gli eventi"; `app/ui/src/components/shell/shell.test.tsx` › "la barra di stato conta gli errori e apre la diagnostica" | coperto |
| Stile visivo | `app/ui/src/theme/tokens.test.ts` › "tema ${name}: tutte le coppie testo/sfondo hanno contrasto >= 4.5:1", › "i due temi definiscono gli stessi token", › "applyTheme scrive le custom properties", › "calcolo del contrasto WCAG" | coperto |
| Primo avvio guidato | Nessun test (`app/ui/src/modules/onboarding` senza test); verifica manuale prevista dal task 13.1, non ancora eseguito | manca |
| Impostazioni | `crates/pc-jobs/tests/manager.rs::profile_change_sets_the_budget_of_later_jobs`; livello di log: `app/ui/src/modules/diagnostics/DiagnosticsPage.test.tsx` › "il livello di log si cambia dalla schermata"; provider: `app/ui/src/modules/models/ModelsPage.test.tsx` › "verifica, test e provider". Nessun test della schermata Impostazioni (`app/ui/src/modules/settings`) | parziale |
| Localizzazione | `app/ui/src/i18n/keys.test.ts` › "trova le chiavi usate nel codice", › "ogni chiave usata esiste in ${lng}" (it, en) | coperto |
| Pubblicazione dell'installer Windows | manuale: release su tag `v*` con installer EXE e checksum verificati, task 14.5 (compilazione setup in CI, task 1.4) | coperto |

## diagnostics-logging

| Requisito | Verifica | Stato |
|---|---|---|
| Log su file con rotazione | `crates/pc-diag/src/rolling.rs::rotates_by_size_and_prunes_oldest`; `crates/pc-diag/src/rolling.rs::rotates_daily`; `crates/pc-diag/src/rolling.rs::resumes_existing_file_after_restart`; `crates/pc-diag/src/lib.rs::writes_json_lines_to_file` | coperto |
| Livello di log configurabile | `crates/pc-diag/src/lib.rs::runtime_level_change_enables_debug`; `crates/pc-diag/src/lib.rs::noisy_libraries_are_capped_at_warn`; `app/ui/src/modules/diagnostics/DiagnosticsPage.test.tsx` › "il livello di log si cambia dalla schermata" | coperto |
| Registro errori strutturato | `crates/pc-store/src/errorlog.rs::warn_with_file_and_phase_is_persisted`; `crates/pc-diag/src/layer.rs::warn_with_fields_becomes_error_record`; `crates/pc-diag/src/layer.rs::errors_before_sink_are_delivered_later` | coperto |
| Messaggi comprensibili | `crates/pc-core/src/error.rs::io_errors_map_to_specific_codes`; `crates/pc-core/src/error.rs::codes_serialize_as_stable_strings`; `app/ui/src/modules/diagnostics/DiagnosticsPage.test.tsx` › "mostra messaggi localizzati, suggerimento e dettaglio tecnico" | coperto |
| Vista diagnostica | `app/ui/src/modules/diagnostics/DiagnosticsPage.test.tsx` › "apre con il filtro del job e applica livello, fase e testo"; `crates/pc-diag/src/layer.rs::tail_is_bounded` | coperto |
| Riprova dei file falliti | `app/src-tauri/src/diagnostics.rs::retry_restricts_each_job_kind_to_failed_files`; `app/ui/src/modules/diagnostics/DiagnosticsPage.test.tsx` › "riprova i file falliti del job filtrato e apre la cartella log" | coperto |
| Pacchetto diagnostico | `crates/pc-diag/src/bundle.rs::explicit_allowlist_excludes_photos_database_crops_and_embedding_fields` | coperto |
| Cattura dei crash | `crates/pc-diag/tests/panic_marker.rs::panic_is_logged_and_marker_survives_restart` (log e marker). L'avviso all'utente al riavvio non è verificato: task 13.4 non eseguito | parziale |
| Storico del progresso | `crates/pc-jobs/tests/phase_logs.rs::ticker_rate_is_limited_and_phase_logs_are_written` | coperto |

## face-recognition

| Requisito | Verifica | Stato |
|---|---|---|
| Rilevamento volti | `crates/pc-ai/tests/models.rs::faces_detected_and_recognized` [model-tests]; `crates/pc-ai/tests/models.rs::usable_faces_are_filtered_embedded_and_saved` [model-tests]; `crates/pc-ai/src/faces.rs::align_maps_template_points_onto_themselves`; `crates/pc-ai/src/faces.rs::similarity_recovers_known_transform` | coperto |
| Raggruppamento in persone | `crates/pc-ai/src/cluster.rs::identities_and_refusals_survive_reclustering` (volti simili nello stesso cluster); `app/ui/src/modules/people/PeoplePage.test.tsx` › "opens a cluster, renames it and sends a persistent refusal". Non verificati il numero di foto per persona e l'etichetta "Senza nome"; task 10.6 non eseguito | parziale |
| Identificazione delle persone | `crates/pc-store/tests/people.rs::new_faces_match_named_centroids_and_respect_refusals`; `crates/pc-ai/tests/models.rs::faces_detected_and_recognized` [model-tests]; `app/ui/src/modules/people/PeoplePage.test.tsx` › "opens a cluster, renames it and sends a persistent refusal" | coperto |
| Correzione dei raggruppamenti | `crates/pc-store/tests/people.rs::corrections_survive_reanalysis_and_removed_faces_stay_removed` (separa, unisci, rimuovi); `crates/pc-ai/src/cluster.rs::persisted_corrections_survive_multiple_clustering_runs`; `crates/pc-ai/src/cluster.rs::identities_and_refusals_survive_reclustering` | coperto |
| Privacy dei dati biometrici | Cancellazione: `crates/pc-store/tests/people.rs::clear_and_vacuum_remove_face_data_from_database_and_wal`; `crates/pc-ai/src/face_policy.rs::clearing_invalidates_queued_results_even_after_reenable`; `crates/pc-ai/src/face_analysis.rs::queued_biometric_results_cannot_reappear_after_purge`; esclusione dal pacchetto: `crates/pc-diag/src/bundle.rs::explicit_allowlist_excludes_photos_database_crops_and_embedding_fields`. Scenario "Riconoscimento disattivato" (niente volti, conteggio persone mantenuto) non verificato | parziale |

## file-organization

| Requisito | Verifica | Stato |
|---|---|---|
| Struttura di cartelle da catalogazioni | `crates/pc-organize/tests/operations.rs::pure_preview_sanitizes_reserved_names_and_conflicts` (livello "luogo", cartella "Senza luogo"). Gerarchia multi-livello (es. Luogo > Anno) non verificata | parziale |
| Metodo di collegamento | `crates/pc-organize/tests/operations.rs::preflight_decision_with_and_without_symlink_privilege` (symlink negati → hard link; volumi diversi → copia); `crates/pc-organize/tests/operations.rs::link_preflight_and_undo_never_remove_link_targets`; `app/ui/src/modules/organize/OrganizePage.test.tsx` › "mostra anteprima con conteggi e spazio, propone l'alternativa e solo dopo avvia" | coperto |
| Anteprima prima dell'esecuzione | `crates/pc-organize/tests/operations.rs::pure_preview_sanitizes_reserved_names_and_conflicts` (nessun IO, conteggi e conflitti); `app/ui/src/modules/organize/OrganizePage.test.tsx` › "mostra anteprima con conteggi e spazio, propone l'alternativa e solo dopo avvia" | coperto |
| Gestione dei conflitti di nome | `crates/pc-organize/tests/operations.rs::pure_preview_sanitizes_reserved_names_and_conflicts` (due `IMG_0001.JPG`); `crates/pc-organize/tests/operations.rs::organize_undo_preserves_originals_existing_files_and_modified_outputs` (file esistente non sovrascritto) | coperto |
| Nomi di cartella validi | `crates/pc-organize/tests/operations.rs::pure_preview_sanitizes_reserved_names_and_conflicts` (`CON`, `NUL.txt`, `LPT9`, `:`, `..`) | coperto |
| Annullamento di un'organizzazione | `crates/pc-organize/tests/operations.rs::organize_undo_preserves_originals_existing_files_and_modified_outputs`; `crates/pc-organize/tests/operations.rs::link_preflight_and_undo_never_remove_link_targets` | coperto |
| Organizzazione come job | `crates/pc-organize/tests/operations.rs::organize_undo_preserves_originals_existing_files_and_modified_outputs` (esecuzione come job). Registrazione dell'errore per singolo file e prosecuzione non verificate | parziale |

## job-orchestration

| Requisito | Verifica | Stato |
|---|---|---|
| Job a fasi | `crates/pc-jobs/src/pipeline.rs::processes_10k_items_through_all_stages`; `crates/pc-ingest/tests/import.rs::full_import_incremental_rescan_moves_and_missing` (fasi nominate dell'import). La sovrapposizione delle fasi (un elemento raggiunge l'AI prima che la scansione finisca) non è asserita | parziale |
| Progresso percentuale per fase e complessivo | `crates/pc-jobs/src/progress.rs::phase_percent_and_indeterminate_total`; `crates/pc-jobs/src/progress.rs::overall_is_weighted`; `crates/pc-jobs/src/progress.rs::empty_total_is_complete`; `app/ui/src/modules/jobs/JobsPage.test.tsx` › "mostra fasi, percentuali, conteggi, ETA e controlli", › "a scansione conclusa mostra la percentuale complessiva" | coperto |
| Velocità ed ETA | `crates/pc-jobs/src/progress.rs::rate_and_eta_after_ten_seconds`; `crates/pc-jobs/src/progress.rs::eta_hidden_before_ten_seconds`; `crates/pc-jobs/src/progress.rs::eta_is_stable_with_bursty_items_and_follows_a_real_slowdown`; `app/ui/src/lib/format.test.ts` › "arrotonda per eccesso con precisione proporzionata alla durata", › "non cambia a ogni secondo durante il conto alla rovescia" | coperto |
| Aggiornamenti di progresso a frequenza limitata | `crates/pc-jobs/tests/phase_logs.rs::ticker_rate_is_limited_and_phase_logs_are_written` | coperto |
| Pausa, ripresa e annullamento | `crates/pc-jobs/tests/manager.rs::pause_resume_and_cancel_keep_completed_results`; `crates/pc-jobs/src/control.rs::checkpoint_blocks_while_paused_and_releases_on_resume_or_cancel`; `crates/pc-jobs/src/pipeline.rs::cancel_stops_promptly_and_keeps_completed_results`; `app/ui/src/modules/jobs/JobsPage.test.tsx` › "job in pausa offre Riprendi; job concluso non ha controlli" | coperto |
| Persistenza e ripresa dei job | `crates/pc-jobs/tests/manager.rs::interrupted_job_resumes_only_pending_items` (`recover_interrupted` + ripresa dei soli pending); `crates/pc-jobs/src/pipeline.rs::journal_skips_finished_items_on_resume` | coperto |
| Errori per file non bloccanti | `crates/pc-jobs/src/pipeline.rs::failures_and_skips_are_counted_and_do_not_stop_the_job`; `crates/pc-jobs/src/pipeline.rs::soft_errors_count_as_failed_but_item_continues`; `crates/pc-jobs/tests/manager.rs::completed_and_completed_with_errors`; `crates/pc-ingest/tests/import.rs::full_import_incremental_rescan_moves_and_missing` | coperto |
| Coda dei job | `crates/pc-jobs/tests/manager.rs::queued_job_can_be_cancelled_and_budget_admission_is_fifo`; `crates/pc-jobs/tests/manager.rs::followup_job_is_queued_after_success_only` | coperto |

## media-ingestion

| Requisito | Verifica | Stato |
|---|---|---|
| Sorgenti di import | `crates/pc-ingest/src/scan.rs::recursion_formats_and_ignore_patterns`; `crates/pc-ingest/src/scan.rs::invalid_pattern_and_missing_root`; `crates/pc-store/src/repo.rs::source_crud_and_cascade`; `app/ui/src/modules/import/ImportPage.test.tsx` › "crea la sorgente con le opzioni scelte e avvia l'import", › "separa i pattern da ignorare" | coperto |
| Formati supportati | `crates/pc-ingest/src/formats.rs::classification`; `crates/pc-ingest/src/metadata.rs::tech_for_every_decodable_format`; `crates/pc-ingest/tests/heif.rs::without_decoder_heic_keeps_metadata_and_reports_unsupported_decode`; `crates/pc-ingest/tests/heif.rs::native_decoder_handles_heic_and_releases_memory_without_touching_source` [heif]; `crates/pc-ingest/src/thumb.rs::invalid_heic_is_reported_and_raw_uses_embedded_preview`; `crates/pc-ingest/tests/raf.rs::raf_container_extracts_preview_and_rejects_out_of_bounds_offsets` | coperto |
| Caratteristiche tecniche | `crates/pc-ingest/src/metadata.rs::tech_for_every_decodable_format`; `crates/pc-ingest/src/metadata.rs::png_dpi_from_phys_and_bit_depth`; `crates/pc-ingest/src/metadata.rs::exif_fields_dates_and_gps` (DPI) | coperto |
| Date di acquisizione e modifica | `crates/pc-ingest/src/metadata.rs::exif_fields_dates_and_gps`; `crates/pc-ingest/src/metadata.rs::missing_exif_falls_back_to_file_system`; `crates/pc-ingest/src/metadata.rs::date_normalization`; `crates/pc-ingest/tests/import.rs::full_import_incremental_rescan_moves_and_missing` | coperto |
| Metadati EXIF | `crates/pc-ingest/src/metadata.rs::exif_fields_dates_and_gps`; `crates/pc-ingest/src/metadata.rs::corrupt_exif_is_reported_with_code`; `crates/pc-store/src/repo.rs::metadata_roundtrip` | coperto |
| Miniature | `crates/pc-ingest/src/thumb.rs::orientation_6_rotates_landscape_to_portrait`; `crates/pc-ingest/src/thumb.rs::jpeg_is_decoded_with_dct_reduction`; `crates/pc-ingest/src/thumb.rs::concurrent_store_of_same_key_succeeds`; `crates/pc-ingest/src/thumb.rs::cache_rejects_unsafe_keys`; `crates/pc-ingest/tests/thumb_memory.rs::thumbnail_of_50mp_jpeg_stays_small` | coperto |
| Originali in sola lettura | `crates/pc-ingest/tests/import.rs::full_import_incremental_rescan_moves_and_missing` (hash e mtime prima/dopo, task 7.9); `crates/pc-ingest/tests/heif.rs::native_decoder_handles_heic_and_releases_memory_without_touching_source` [heif] | coperto |
| Scansione incrementale | `crates/pc-ingest/tests/import.rs::full_import_incremental_rescan_moves_and_missing` (rescan con 0 rielaborati, file spostato, file mancante) | coperto |
| Rilevamento duplicati | `crates/pc-ingest/src/hash.rs::same_content_same_hashes_different_content_differs`; `crates/pc-store/src/repo.rs::duplicates_and_relink`; `crates/pc-ingest/tests/import.rs::full_import_incremental_rescan_moves_and_missing` | coperto |
| Monitoraggio delle sorgenti | `crates/pc-ingest/tests/import.rs::watched_source_imports_new_files_automatically` | coperto |

## model-management

| Requisito | Verifica | Stato |
|---|---|---|
| Manifest dei modelli | `crates/pc-models/src/manifest.rs::builtin_manifest_is_valid_and_complete`; `crates/pc-models/src/manifest.rs::invalid_manifests_are_rejected`; `crates/pc-models/src/store.rs::new_manifest_version_shows_update_available`; `crates/pc-models/src/store.rs::a_new_manifest_path_cannot_reuse_another_files_cached_hash` | coperto |
| Verifica e auto-download all'avvio | `crates/pc-models/src/store.rs::missing_model_is_restored_from_bundle_or_downloaded`; `crates/pc-models/src/store.rs::corrupt_file_on_disk_is_detected_and_replaced`; `crates/pc-ai/tests/selftest.rs::bundled_models_restore_and_self_test_as_a_background_job` [model-tests] | coperto |
| Download robusto | `crates/pc-models/src/download.rs::resumes_after_interruption_with_range`; `crates/pc-models/src/download.rs::corrupt_download_is_discarded_then_mirror_succeeds`; `crates/pc-models/src/download.rs::permanent_http_error_moves_to_mirror_and_reports_failure`; `crates/pc-models/src/download.rs::failed_transfer_can_resume_in_a_later_job`; `crates/pc-models/src/download.rs::cancellation_keeps_partial_file_and_never_publishes_it`; `crates/pc-models/src/download.rs::incorrect_range_is_discarded_before_restarting`; `crates/pc-models/src/store.rs::failed_download_marks_error_and_local_install_checks_hash` | coperto |
| Installazione offline | `crates/pc-models/src/store.rs::failed_download_marks_error_and_local_install_checks_hash` (hash errato rifiutato, file corretto verificato); `app/ui/src/modules/models/ModelsPage.test.tsx` › "installa da file e mostra gli errori dei comandi globali" | coperto |
| Self-test di funzionamento | `crates/pc-ai/tests/selftest.rs::every_bundled_model_passes_its_self_test` [model-tests]; `crates/pc-ai/tests/selftest.rs::altered_model_fails_self_test` [model-tests]; `crates/pc-models/src/store.rs::self_test_result_survives_restart_and_failure_blocks_use`; `crates/pc-models/src/pool.rs::failed_self_test_blocks_an_already_cached_session`; `crates/pc-ai/src/models_job.rs::unknown_model_is_rejected_and_missing_self_test_is_a_job_failure` | coperto |
| Provider di esecuzione | `crates/pc-models/src/runtime.rs::loads_runtime_and_runs_on_cpu`; `crates/pc-models/src/runtime.rs::auto_uses_directml_when_available`; `crates/pc-models/src/runtime.rs::gpu_failure_falls_back_to_cpu`; `crates/pc-models/src/store.rs::crash_during_gpu_trial_disables_gpu_for_that_model`; `app/ui/src/modules/models/ModelsPage.test.tsx` › "verifica, test e provider" | coperto |
| Stato dei modelli visibile | `app/ui/src/modules/models/ModelsPage.test.tsx` › "mostra sistema, stati, self-test e aggiorna il download dagli eventi", › "mostra stati mancanti, aggiornamenti e test falliti con azioni coerenti"; `crates/pc-models/src/store.rs::corrupt_file_does_not_show_a_previous_successful_test`; `crates/pc-models/src/store.rs::restoring_verified_bytes_does_not_clear_a_failed_self_test` | coperto |

## resource-governor

| Requisito | Verifica | Stato |
|---|---|---|
| Rilevamento risorse all'avvio | `crates/pc-governor/src/lib.rs::detect_logs_system_resources`; `crates/pc-governor/src/probe.rs::real_probe_values_are_plausible`; acceleratori: `crates/pc-models/src/runtime.rs::auto_uses_directml_when_available`; schermata di sistema: `app/ui/src/modules/models/ModelsPage.test.tsx` › "mostra sistema, stati, self-test e aggiorna il download dagli eventi" | coperto |
| Budget di memoria | `crates/pc-governor/src/budget.rs::budget_matrix` (4/8/16/32 GB); `crates/pc-governor/src/budget.rs::user_limit_always_caps` | coperto |
| Requisiti minimi | Nessun test che, con RAM insufficiente per il modello più piccolo, disattivi le analisi AI con avviso mantenendo import/libreria/export funzionanti | manca |
| Ammissione dei lavori in base al budget | `crates/pc-governor/src/governor.rs::concurrent_estimated_use_never_exceeds_budget`; `crates/pc-governor/src/governor.rs::oversized_cost_runs_alone`; `crates/pc-governor/src/budget.rs::workers_and_batch_respect_limits`; `crates/pc-governor/src/budget.rs::decode_cost_scales_with_dct_reduction`; `crates/pc-jobs/tests/manager.rs::queued_job_can_be_cancelled_and_budget_admission_is_fifo` | coperto |
| Residenza dei modelli | `crates/pc-models/src/pool.rs::models_that_do_not_fit_together_are_swapped`; `crates/pc-models/src/pool.rs::models_that_fit_stay_resident_together`; `crates/pc-models/src/pool.rs::measured_cost_above_budget_releases_session_and_reservation`; `crates/pc-governor/src/governor.rs::residents_count_against_capacity`; `crates/pc-ai/tests/models.rs::model_major_releases_each_session_before_loading_next_pass` [model-tests] | coperto |
| Adattamento alla pressione di memoria | `crates/pc-governor/src/monitor.rs::transitions_reduce_capacity_and_pause_ai`; `crates/pc-governor/src/monitor.rs::transitions_are_logged`; `crates/pc-governor/src/governor.rs::reduced_capacity_blocks_new_permits_until_released`; `crates/pc-governor/src/governor.rs::reduced_buffer_releases_temporary_memory_quota` | coperto |
| Visibilità del consumo | `crates/pc-governor/src/monitor.rs::snapshot_payload_and_frequency`; `app/ui/src/components/shell/shell.test.tsx` › "pill del job e chip RAM seguono gli eventi"; `app/ui/src/modules/jobs/JobsPage.test.tsx` › "pannello risorse con budget e modelli residenti" | coperto |

## Lacune

Requisiti senza verifica (**manca**) e con verifica **parziale**, con il test che chiuderebbe ciascuna lacuna.

### Mancanti

1. **ai-classification / Esecuzione esclusivamente locale**: eseguire il task 14.2 (analisi completa con monitor di rete, nessuna connessione). In aggiunta, un test di integrazione `pc-ai` [model-tests] che esegue la pipeline di analisi con un proxy/variabili d'ambiente che rendono irraggiungibile la rete e verifica che l'analisi termini; oppure un test che controlla che il grafo delle dipendenze di `pc-ai` non includa client HTTP.
2. **desktop-shell / Primo avvio guidato**: test di componente per `app/ui/src/modules/onboarding` che percorre benvenuto → verifica sistema → download modelli → prima sorgente e verifica che "Rimanda" sul download porti alla libreria con il download offerto in Modelli AI; più la verifica manuale del task 13.1 su profilo pulito.
3. **resource-governor / Requisiti minimi**: unit test in `pc-governor`/`pc-ai` con budget inferiore al costo del modello più piccolo nel manifest: le fasi AI vengono disattivate con un avviso (codice `INSUFFICIENT_MEMORY` nel registro), mentre un import di prova termina con metadati, miniature e libreria popolati.

### Parziali

1. **bulk-operations / Azioni bulk disponibili**: test in `crates/pc-jobs/src/bulk.rs` per le azioni `assign_person` e `reanalyze` su una selezione (assegnazione persistita; job di rianalisi accodato solo per gli id selezionati).
2. **bulk-operations / Conferma per azioni distruttive**: test backend di "rimuovi dalla libreria" su file reali in una cartella temporanea che verifica rimozione dal DB e file originali intatti (hash e mtime).
3. **catalog-export / File strutturato di catalogazione**: test di export con i campi predefiniti che verifica la presenza di tutte le colonne richieste (percorso, nome, formato, w/h, DPI x/y, bit, dimensione, date, luogo, catalogazioni, persone, tag).
4. **catalog-export / Ambito e campi selezionabili**: test con `Scope::Catalog` (solo i file del catalogo) e con il campo confidenza incluso.
5. **catalog-library / Tag utente**: test che assegna un tag, esegue `analysis::classification` (rianalisi) e verifica che i tag utente restino invariati e separati dalle etichette AI.
6. **desktop-shell / Moduli di navigazione**: test su `app/ui/src/modules/registry.ts` che verifica la presenza dei 9 moduli e che la selezione dalla nav mostri il modulo corrispondente.
7. **desktop-shell / Pannello di ispezione**: test di componente che modifica una catalogazione, aggiunge un tag e un catalogo dall'ispettore e verifica i comandi IPC inviati.
8. **desktop-shell / Impostazioni**: test di componente per `app/ui/src/modules/settings` (profilo/limite RAM, soglie, volti on/off, tema, lingua, percorso dati) che verifica il salvataggio della configurazione.
9. **diagnostics-logging / Cattura dei crash**: test di componente che, con marker `last_session_crashed` presente, mostra l'avviso con collegamento alla Diagnostica; più la verifica manuale del task 13.4.
10. **face-recognition / Raggruppamento in persone**: test di `PeoplePage` (task 10.6) che verifica il numero di foto per persona e l'etichetta "Senza nome" per i cluster senza nome; verifica manuale su libreria di prova.
11. **face-recognition / Privacy dei dati biometrici**: test con `FacePolicy::new(false)` che esegue l'analisi e verifica nessun volto/embedding salvato e `media_people.count` comunque valorizzato.
12. **file-organization / Struttura di cartelle da catalogazioni**: test di anteprima con `levels: ["place", "year"]` che verifica percorsi `Luogo/Anno/...`.
13. **file-organization / Organizzazione come job**: test con un file sorgente reso illeggibile o rimosso: il job termina "Completato con errori", l'errore è nel registro con percorso e gli altri file sono creati.
14. **job-orchestration / Job a fasi**: test in `crates/pc-jobs/src/pipeline.rs` in cui la sorgente attende un segnale emesso dallo stadio successivo dopo il primo elemento, dimostrando l'elaborazione sovrapposta.

### Osservazioni minori (non conteggiate)

- Date: il ramo di fallback `CreateDate` (`DateSource::ExifCreate`) non ha un test dedicato.
- Caratteristiche tecniche: lo spazio colore non è asserito.
- Nomi di cartella validi: il limite di lunghezza non è asserito.
- Coda dei job: la visualizzazione dei job "in coda" nella schermata Job non ha un test di componente.
- I test [model-tests] e [heif] non girano nella suite predefinita: la copertura relativa dipende dall'esecuzione con le feature attive.
