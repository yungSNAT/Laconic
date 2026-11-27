//! Where a reference definition may start, checked against CommonMark.

use super::*;

#[test]
fn a_definition_cannot_interrupt_a_paragraph() {
    let root = tree("interrupt", &[("README.md", ""), ("x.md", "")]);
    let workspace = Workspace::new(&root).unwrap();
    let folder = workspace.folder_of(&root.join("README.md")).unwrap();
    let place = Some((&workspace, folder.as_path()));
    // A line shaped like a definition inside a paragraph is prose, and defines nothing.
    assert!(names("Intro line.\n[Note]: read this first.\n", place).is_empty());
    assert_eq!(
        names("Intro line.\n[Note]: x.md\n\nSee [a][note].\n", place),
        ["broken-anchor"]
    );
    // Definitions may follow one another, with a title or target on the next line.
    let text = "[a]: x.md\n  \"Title\"\n[b]:\nx.md\n[c]: gone.md\n\n[1][a] [2][b]\n";
    assert_eq!(names(text, place), ["broken-link"]);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_definition_opens_only_after_a_block_that_has_ended() {
    let root = tree("block-end", &[("README.md", "")]);
    let workspace = Workspace::new(&root).unwrap();
    let folder = workspace.folder_of(&root.join("README.md")).unwrap();
    let place = Some((&workspace, folder.as_path()));
    // The line continues a paragraph, a table, a quotation, an HTML block of type 6 or 7, or
    // one of types 1 to 5 that a blank line does not end.
    for before in [
        "<b>Note</b> text",
        "| a | b |",
        "| a |\n| - |",
        "> Quoted",
        "Hard break\\",
        "Hard break  ",
        "<div>",
        "<b>",
        "<!-- open",
        "<!-- open\n",
        "<script>\n\n",
        "#hashtag",
    ] {
        let text = format!("{before}\n[Note]: read this.\n");
        assert!(names(&text, place).is_empty(), "{before:?}");
    }
    // A heading, thematic break, setext underline, fence, indented code or closed HTML block
    // of types 1 to 5 has ended, so the next line may define.
    for before in [
        "# Heading",
        "***",
        "Title\n===",
        "```\ncode\n```",
        "    code",
        "<!-- note -->",
        "<script>x</script>",
        "<?php x ?>",
        "<!DOCTYPE html>",
        "<![CDATA[x]]>",
        "<script>\nx = 1;\n</script>",
        "<pre>\n</PRE>",
        "<!--\nLinks\n-->",
    ] {
        let text = format!("{before}\n[a]: gone.md\n");
        assert_eq!(names(&text, place), ["broken-link"], "{before:?}");
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn definitions_after_blocks_read_as_commonmark_does() {
    let root = tree("probes", &[("README.md", ""), ("x.md", "# X\n")]);
    let workspace = Workspace::new(&root).unwrap();
    let folder = workspace.folder_of(&root.join("README.md")).unwrap();
    let place = Some((&workspace, folder.as_path()));
    // Each input's result as a CommonMark renderer reads its definitions.
    for (text, expected) in [
        ("<b>Note</b> text\n[Note]: read this.\n", &[][..]),
        ("| a | b |\n[Note]: read this.\n", &[]),
        (
            "<div>\nx\n</div>\n\n[a]: gone.md\n\n[x][a]\n",
            &["broken-link"],
        ),
        ("<!-- note -->\n[a]: gone.md\n\n[x][a]\n", &["broken-link"]),
        (
            "---\ntitle: x\n---\n[a]: gone.md\n\n[x][a]\n",
            &["broken-link"],
        ),
        ("- item\n[Note]: read this.\n", &[]),
        ("```\ncode\n```\n[a]: gone.md\n\n[x][a]\n", &["broken-link"]),
        (
            "Para\n<!-- c -->\n[a]: gone.md\n\n[x][a]\n",
            &["broken-link"],
        ),
        ("Para\n<div>\n[Note]: read this.\n", &[]),
        (
            "<PRE>x</script>\n[a]: gone.md\n\n[x][a]\n",
            &["broken-link"],
        ),
        ("    <!-- x -->\n[a]: gone.md\n\n[x][a]\n", &["broken-link"]),
        ("# Heading\n[a]: gone.md\n\n[x][a]\n", &["broken-link"]),
        ("Title\n===\n[a]: gone.md\n\n[x][a]\n", &["broken-link"]),
        ("> Quoted\n[Note]: read this.\n", &[]),
        ("<div>\n</div>\n[Note]: read this.\n", &[]),
        (
            "[a]: x.md\n[b]: gone.md\n\n[1][a] [2][b]\n",
            &["broken-link"],
        ),
        (
            "[a]: x.md\n(Title)\n[b]: gone.md\n\n[2][b]\n",
            &["broken-link"],
        ),
        (
            "[a]: x.md\nProse here.\n[b]: gone.md\n\n[2][b]\n",
            &["broken-anchor"],
        ),
        ("Intro line.\n[Note]: read this first.\n", &[]),
        ("[t][a]\n\n[a]:\nmissing.md\n", &["broken-link"]),
        ("[a](<#x\ny>)\n", &[]),
        ("## The `&nbsp;` entity\n\n[e](#the-nbsp-entity)\n", &[]),
        ("[x](notes[1].md)\n", &["broken-link"]),
        ("   [a]: gone.md\n\n[1][a]\n", &["broken-link"]),
        // An HTML block left open in an alert or a list item ends with it.
        (
            "> [!NOTE]\n> <pre>\n\nPara.\n\n[a]: gone.md\n\n[x][a]\n",
            &["broken-link"],
        ),
        (
            "- item\n\n  <pre>\n  x\n\nPara.\n\n[a]: gone.md\n\n[x][a]\n",
            &["broken-link"],
        ),
        (
            "- item\n\n<script>\nx\n</script>\n[a]: gone.md\n",
            &["broken-link"],
        ),
        // A block that opens and closes on one line ends there, in a container too.
        (
            "> [!NOTE]\n> <!-- c -->\n[a]: gone.md\n\n[x][a]\n",
            &["broken-link"],
        ),
        (
            "- item\n\n  <!-- c -->\n[a]: gone.md\n\n[x][a]\n",
            &["broken-link"],
        ),
    ] {
        assert_eq!(names(text, place), expected, "{text:?}");
    }
    let _ = std::fs::remove_dir_all(&root);
}
