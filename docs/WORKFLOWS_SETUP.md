# GitHub Workflows Setup Guide

This guide will help you set up the automated workflows for the TP-Lib project.

## Prerequisites

Before the workflows can run successfully, you need to:

1. **GitHub Repository Settings**
2. **OIDC Trusted Publishers Configuration**
3. **GitHub Pages Setup**
4. **Branch Protection Rules (optional but recommended)**

## 1. GitHub Pages Setup

Enable GitHub Pages to deploy documentation automatically.

### Steps:

1. Go to your repository on GitHub
2. Click **Settings** → **Pages** (in the left sidebar)
3. Under **Build and deployment**:
   - **Source**: Select **GitHub Actions**
4. Save the changes

The documentation will be available at: `https://matdata-eu.github.io/tp-lib/`

## 2. Configure OIDC Trusted Publishers

The publishing workflows use OpenID Connect (OIDC) for secure authentication without storing API tokens. You need to configure both crates.io and PyPI to trust GitHub Actions from this repository.

### 2.1 crates.io Trusted Publisher

**Purpose:** Allow GitHub Actions to publish Rust crates without API tokens

**Steps:**

1. Go to https://crates.io/settings/tokens
2. Log in with your GitHub account
3. Scroll to **Trusted Publishing** section
4. Click **Add trusted publisher**
5. Fill in the form:
   - **Repository Owner:** `matdata-eu`
   - **Repository Name:** `tp-lib`
   - **Workflow:** `.github/workflows/publish-crates.yml`
6. Click **Add publisher**

**What this does:**
- Allows the specified workflow to publish crates on your behalf
- No API token needed in GitHub secrets
- Tokens are automatically generated and short-lived
- More secure than storing long-lived tokens

### 2.2 PyPI Trusted Publisher

**Purpose:** Allow GitHub Actions to publish Python packages without API tokens

**Steps:**

1. Go to https://pypi.org/manage/account/publishing/
2. Log in to your PyPI account
3. Under **Add a new publisher**, fill in:
   - **PyPI Project Name:** `tp-lib`
   - **Owner:** `matdata-eu`
   - **Repository name:** `tp-lib`
   - **Workflow name:** `publish-pypi.yml`
   - **Environment name:** `pypi`
4. Click **Add**

**Important Notes:**

- **For first-time publishing:** If the `tp-lib` project doesn't exist on PyPI yet, you have two options:
  1. **Option A (Recommended):** Use a temporary API token for the first release, then switch to trusted publishing
  2. **Option B:** Pre-register the project name on PyPI, then configure trusted publishing before the first release

- **After first publish:** All subsequent releases will automatically use trusted publishing with OIDC

**What this does:**
- Allows the specified workflow to publish packages on your behalf
- No API token needed in GitHub secrets
- Automatically authenticates using GitHub's OIDC provider
- More secure and no token rotation needed

### 2.3 Verify Trusted Publisher Configuration

After configuration, you can verify:

**crates.io:**
- Visit https://crates.io/settings/tokens
- You should see your repository listed under "Trusted Publishers"

**PyPI:**
- Visit https://pypi.org/manage/account/publishing/
- You should see your repository listed under "Trusted Publishers"

## 3. Branch Protection Rules (Recommended)

Protect your `main` branch to ensure code quality.

### Steps:

1. Go to **Settings** → **Branches**
2. Click **Add branch protection rule**
3. Branch name pattern: `main`
4. Enable these options:

**Required checks:**
- ✅ Require status checks to pass before merging
- ✅ Require branches to be up to date before merging
- Select required checks:
  - `Test Suite`
  - `Python Tests`
  - `Linting`
  - `License & Security Check`

**Merge restrictions:**
- ✅ Require linear history
- ✅ Do not allow bypassing the above settings (include administrators)

5. Click **Save changes**

## 4. Verify Workflows

After setup, verify everything works:

### 4.1 Test CI Workflow

1. Create a test branch:
   ```bash
   git checkout -b test-ci
   echo "# Test" >> README.md
   git add README.md
   git commit -m "test: trigger CI"
   git push origin test-ci
   ```

2. Go to **Actions** tab on GitHub
3. You should see "CI" workflow running
4. Wait for all jobs to complete (✅ green checks)

### 4.2 Test Documentation Deployment

1. Merge your test branch to `main` (or push directly):
   ```bash
   git checkout main
   git merge test-ci
   git push origin main
   ```

2. Go to **Actions** tab → **Deploy Documentation** workflow
3. Wait for it to complete
4. Visit: `https://matdata-eu.github.io/tp-lib/`
5. You should see the documentation with links to all crates

### 4.3 Test Release Publishing (when ready)

⚠️ **Warning:** This will publish to crates.io and PyPI! Only do this when ready for a real release.

1. Update versions in all `Cargo.toml` files:
   ```bash
   # Update workspace Cargo.toml, tp-core/Cargo.toml, tp-cli/Cargo.toml, tp-py/Cargo.toml
   # Example: version = "0.1.0"
   ```

2. Update `tp-py/pyproject.toml`:
   ```toml
   version = "0.1.0"
   ```

3. Commit and push:
   ```bash
   git add .
   git commit -m "chore: release v0.1.0"
   git push origin main
   ```

4. Create and push tag:
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

5. Create GitHub Release:
   - Go to: https://github.com/matdata-eu/tp-lib/releases/new
   - Tag: `v0.1.0`
   - Title: `Release 0.1.0`
   - Description: Add release notes
   - Click **Publish release**

6. Monitor workflows:
   - **Actions** tab → **Publish to crates.io**
   - **Actions** tab → **Publish to PyPI**

7. Verify publication:
   - crates.io: https://crates.io/crates/tp-core
   - PyPI: https://pypi.org/project/tp-lib/

## 5. Troubleshooting

### CI fails with "cargo deny check"

**Problem:** License or security issues detected

**Solution:**
- Check workflow logs for specific errors
- Review [SECURITY.md](../SECURITY.md) for known issues
- Run locally: `cargo deny check`
- Update dependencies or add exceptions to `deny.toml`

### Documentation not deploying

**Problem:** GitHub Pages not enabled or workflow fails

**Solutions:**
- Verify GitHub Pages is set to "GitHub Actions" source
- Check workflow logs in Actions tab
- Ensure workflow has `pages: write` permission (already configured)
- Try manual trigger: Actions → Deploy Documentation → Run workflow

### crates.io publish fails

**Problem:** Authentication or publishing error

**Solutions:**
- Verify trusted publisher is configured on crates.io
- Check that repository owner, name, and workflow path match exactly
- Ensure the workflow has `id-token: write` permission (already configured)
- Review workflow logs for specific error messages
- For first-time publishing, you may need to create the crate first

### PyPI publish fails

**Problem:** Authentication or publishing error

**Solutions:**
- Verify trusted publisher is configured on PyPI
- Check that project name, owner, repository, workflow, and environment match exactly
- Ensure the workflow has `id-token: write` permission (already configured)
- For first publish, consider using a temporary API token or pre-registering the project name
- Review workflow logs for specific error messages

### Version mismatch error in release

**Problem:** Git tag version doesn't match Cargo.toml

**Solution:**
```bash
# Check versions match
TAG_VERSION="0.1.0"
grep "version = " Cargo.toml

# Fix if needed, then:
git tag -d v${TAG_VERSION}
git push origin :refs/tags/v${TAG_VERSION}

# Update Cargo.toml files, commit, then:
git tag v${TAG_VERSION}
git push origin v${TAG_VERSION}
```

### Wheel build fails on specific platform

**Problem:** Platform-specific compilation errors

**Solutions:**
- Check which platform failed in workflow logs
- May need platform-specific dependencies
- Consider using `maturin` build options
- Test locally with: `maturin build --release`

## 6. Monitoring

Keep track of your automation:

### Workflow Status
- **Dashboard:** https://github.com/matdata-eu/tp-lib/actions
- **CI Status:** Check badge in README
- **Email Notifications:** GitHub sends emails on workflow failures

### Published Packages
- **crates.io:** https://crates.io/crates/tp-core
- **PyPI:** https://pypi.org/project/tp-lib/
- **Documentation:** https://matdata-eu.github.io/tp-lib/

### Download Statistics
- **crates.io stats:** Available on crate page
- **PyPI stats:** https://pypistats.org/packages/tp-lib

## 7. Updating Workflows

If you need to modify workflows:

1. Edit files in `.github/workflows/`
2. Test changes by pushing to a test branch
3. Review workflow run in Actions tab
4. Merge to main when working

**Key workflow files:**
- `ci.yml` - Continuous integration
- `publish-crates.yml` - Rust crate publishing
- `publish-pypi.yml` - Python package publishing
- `docs.yml` - Documentation deployment

## 8. Security Best Practices

### OIDC Trusted Publishing
- ✅ Use OIDC trusted publishing (more secure than API tokens)
- ✅ Verify trusted publisher configuration on both platforms
- ✅ No secrets to manage or rotate
- ✅ Short-lived, automatically rotated tokens
- ✅ Scoped to specific workflows and repositories

### Workflow Security
- ✅ Pin action versions (e.g., `@v4` not `@main`)
- ✅ Review workflow logs for sensitive data exposure
- ✅ Use `GITHUB_TOKEN` for GitHub API calls (already configured)
- ✅ Enable branch protection to prevent unauthorized releases
- ✅ Use environment protection rules for additional security (optional)

## Next Steps

After completing this setup:

1. ✅ Verify all workflows run successfully
2. ✅ Update repository URLs in workflows and README
3. ✅ Test the full release process with a test version (e.g., v0.0.1-test)
4. ✅ Document your release process in CONTRIBUTING.md
5. ✅ Set up notifications for workflow failures
6. ✅ Consider adding badges to README (already done)

For detailed workflow documentation, see [WORKFLOWS.md](WORKFLOWS.md).

---

**Questions?** Open an issue on GitHub or check the workflow logs for error details.
