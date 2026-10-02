#!/usr/bin/env python3
"""Publish the Dopbase starter pack with GitHub CLI authentication."""

import argparse
import json
import re
import subprocess
from pathlib import Path

DIRECTORY = Path(__file__).resolve().parent
MANIFEST = DIRECTORY / "manifest.json"


def graphql(query, **variables):
    payload = json.dumps({"query": query, "variables": variables})
    result = subprocess.run(
        ["gh", "api", "graphql", "--input", "-"],
        input=payload,
        text=True,
        capture_output=True,
        check=True,
    )
    response = json.loads(result.stdout)
    if response.get("errors"):
        raise RuntimeError(json.dumps(response["errors"]))
    return response["data"]


def save(manifest):
    # Save after each creation so a partially completed run can resume.
    temporary = MANIFEST.with_suffix(".json.tmp")
    temporary.write_text(json.dumps(manifest, indent=2) + "\n")
    temporary.replace(MANIFEST)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--apply", action="store_true", help="Publish and update posts.")
    parser.add_argument(
        "--allow-existing-categories",
        action="store_true",
        help="Use the documented existing-category mapping until web setup is complete.",
    )
    args = parser.parse_args()
    manifest = json.loads(MANIFEST.read_text())
    owner, name = manifest["repository"].split("/")
    repo = graphql(
        """query($owner:String!,$name:String!){repository(owner:$owner,name:$name){
        id discussionCategories(first:25){nodes{id name slug}}
        }}""",
        owner=owner,
        name=name,
    )["repository"]
    categories = {c["name"]: c for c in repo["discussionCategories"]["nodes"]}
    discussions = []
    cursor = None
    while True:
        page = graphql(
            """query($owner:String!,$name:String!,$cursor:String){
            repository(owner:$owner,name:$name){discussions(first:100,after:$cursor){
            nodes{id number title url body category{id name slug}}
            pageInfo{hasNextPage endCursor}}}}""",
            owner=owner,
            name=name,
            cursor=cursor,
        )["repository"]["discussions"]
        discussions.extend(page["nodes"])
        if not page["pageInfo"]["hasNextPage"]:
            break
        cursor = page["pageInfo"]["endCursor"]
    by_number = {d["number"]: d for d in discussions}
    by_title = {d["title"]: d for d in discussions}
    selected = {}
    missing = []
    for post in manifest["posts"]:
        category = categories.get(post["desired_category"])
        if category is None:
            missing.append(post["desired_category"])
            if args.allow_existing_categories:
                category = categories.get(post["fallback_category"])
        if category is None:
            continue
        selected[post["key"]] = category
    if missing:
        print("Categories awaiting web setup: " + ", ".join(sorted(set(missing))))
    if len(selected) != len(manifest["posts"]):
        raise SystemExit("Complete the category checklist or explicitly allow existing categories.")
    for post in manifest["posts"]:
        existing = by_number.get(post["number"]) or by_title.get(post["title"])
        if existing:
            post.update(number=existing["number"], url=existing["url"], id=existing["id"])
        print(("Update" if existing else "Create") + ": " + post["title"] + " -> " + selected[post["key"]]["name"])
    if not args.apply:
        print("Preview only. No posts or files changed.")
        return

    def render(post):
        source = (DIRECTORY / post["file"]).read_text()
        _, separator, body = source.partition("\n")
        if not separator:
            raise ValueError("Post has no body: " + post["file"])
        urls = {p["key"] + "_url": p.get("url") or f"https://github.com/{owner}/{name}/discussions" for p in manifest["posts"]}
        body = re.sub(r"\{\{([a-z_]+)\}\}", lambda match: urls[match[1]], body.strip())
        slug_map = {"q-a": "Help & questions", "ideas": "Feature requests", "show-and-tell": "Showcase"}
        for old_slug, desired in slug_map.items():
            actual = categories.get(desired)
            if actual:
                body = body.replace("/discussions/categories/" + old_slug, "/discussions/categories/" + actual["slug"])
        if re.search(r"\{\{[^}]+\}\}", body):
            raise ValueError("Unresolved post reference")
        return body

    order = ["install", "import", "config", "feedback", "features", "showcase", "general", "welcome"]
    for key in order:
        post = next(p for p in manifest["posts"] if p["key"] == key)
        if not post.get("id"):
            created = graphql(
                """mutation($input:CreateDiscussionInput!){createDiscussion(input:$input){discussion{id number url}}}""",
                input={"repositoryId": repo["id"], "categoryId": selected[key]["id"], "title": post["title"], "body": render(post)},
            )["createDiscussion"]["discussion"]
            post.update(created)
            post["actual_category"] = selected[key]["name"]
            post["actual_category_slug"] = selected[key]["slug"]
            save(manifest)

    for post in manifest["posts"]:
        body = render(post)
        category = selected[post["key"]]
        current = by_number.get(post["number"])
        if not current or current["body"] != body or current["title"] != post["title"] or current["category"]["id"] != category["id"]:
            graphql(
                """mutation($input:UpdateDiscussionInput!){updateDiscussion(input:$input){discussion{id number url}}}""",
                input={"discussionId": post["id"], "categoryId": category["id"], "title": post["title"], "body": body},
            )
        post["actual_category"] = category["name"]
        post["actual_category_slug"] = category["slug"]
        # Keep repository copies readable without requiring the publication helper.
        (DIRECTORY / post["file"]).write_text("# " + post["title"] + "\n\n" + body + "\n")
        save(manifest)
    manifest["publication_status"] = "published-awaiting-category-and-pin-setup" if missing else "published-awaiting-pin-verification"
    save(manifest)
    print("Published " + str(len(manifest["posts"])) + " posts. Pins and sections require web setup.")


if __name__ == "__main__":
    main()
