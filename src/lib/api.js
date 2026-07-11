// Thin wrappers over the Rust commands. Keeping the invoke names in one place
// means the rest of the app never touches Tauri directly.
import { invoke } from "@tauri-apps/api/core";

export const loadConfig = () => invoke("load_config");
export const saveConfig = (config) => invoke("save_config", { config });

export const writeSession = (session) => invoke("write_session", { session });
export const listSessions = () => invoke("list_sessions");

export const previewSubject = (subjectId, subjectName) =>
  invoke("preview_subject", { subjectId, subjectName });

export const exportSubject = (subjectId, subjectName, destDir) =>
  invoke("export_subject", { subjectId, subjectName, destDir });

export const exportAll = (subjects, destDir) =>
  invoke("export_all", { subjects, destDir });

export const dataDir = () => invoke("data_dir");
