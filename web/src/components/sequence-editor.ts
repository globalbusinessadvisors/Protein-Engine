/**
 * Sequence editor component — text input for amino acid sequences.
 *
 * Placeholder: full implementation will provide validation, highlighting,
 * and integration with the WASM scoring pipeline.
 */

const AMINO_ACIDS = "ACDEFGHIKLMNPQRSTVWY";

export function isValidSequence(seq: string): boolean {
  return seq.length > 0 && [...seq.toUpperCase()].every((c) => AMINO_ACIDS.includes(c));
}

export function createSequenceEditor(
  container: HTMLElement,
  onSubmit: (sequence: string) => void,
): void {
  const textarea = document.createElement("textarea");
  textarea.rows = 4;
  textarea.cols = 60;
  textarea.placeholder = "Enter amino acid sequence (e.g. MAGHLASDFAF...)";
  textarea.style.fontFamily = "monospace";

  const button = document.createElement("button");
  button.textContent = "Score";
  button.addEventListener("click", () => {
    const seq = textarea.value.trim().toUpperCase();
    if (isValidSequence(seq)) {
      onSubmit(seq);
    }
  });

  container.appendChild(textarea);
  container.appendChild(document.createElement("br"));
  container.appendChild(button);
}
