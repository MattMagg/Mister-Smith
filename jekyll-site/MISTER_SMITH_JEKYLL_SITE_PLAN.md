# Mister Smith Jekyll GitHub Pages Site - Foundation Setup Plan

## Foundation Objectives

Establish basic Jekyll site skeleton and infrastructure to prepare for design and content implementation:
- Initialize Jekyll site with proper structure
- Configure collections for documentation domains
- Set up basic theme framework
- Establish build and deployment pipeline
- Create foundation for future design and content work

## Technical Architecture

### Jekyll Site Structure
```
/
├── _config.yml                 # Main Jekyll configuration
├── Gemfile                     # Ruby dependencies
├── _layouts/                   # Custom layouts
│   ├── default.html           # Base layout with navigation
│   ├── documentation.html     # Technical documentation layout
│   ├── collection.html        # Collection index layout
│   └── home.html             # Homepage layout
├── _includes/                  # Reusable components
│   ├── navigation.html        # Main navigation bar
│   ├── sidebar.html          # Section-specific sidebar
│   ├── breadcrumbs.html      # Breadcrumb navigation
│   ├── toc.html              # Auto-generated table of contents
│   └── search.html           # Search functionality
├── _sass/                     # Custom SCSS styles
│   ├── _variables.scss       # Design system variables
│   ├── _navigation.scss      # Navigation styles
│   └── _documentation.scss   # Documentation-specific styles
├── assets/                    # Static assets
│   ├── css/main.scss         # Main stylesheet
│   ├── js/                   # JavaScript functionality
│   └── images/               # Site images and icons
├── _core_architecture/        # Collection: Core architecture docs
├── _data_management/          # Collection: Data management docs
├── _operations/               # Collection: Operations docs
├── _research/                 # Collection: Research docs
├── _security/                 # Collection: Security docs
├── _transport/                # Collection: Transport docs
├── _data/                     # Site data files
│   ├── navigation.yml         # Navigation structure
│   └── collections.yml        # Collection metadata
├── index.md                   # Homepage
└── search.md                  # Search results page
```

### Theme Selection: Minimal Mistakes

Technical requirements:

- Documentation support with built-in features
- Responsive design optimized for technical content
- GitHub Pages compatible
- Built-in search functionality
- Sidebar navigation support

### Collections Configuration

Jekyll collections for documentation domains:

- `core_architecture` - System architecture, module organization
- `data_management` - Agent orchestration, data persistence
- `operations` - Observability, monitoring, deployment
- `research` - Claude CLI integration, analysis
- `security` - Security framework and patterns
- `transport` - Transport layer specifications

## Foundation Implementation Steps

### Step 1: Jekyll Site Initialization

Tasks:

1. Initialize blank Jekyll site structure
2. Configure basic Gemfile with essential dependencies
3. Set up minimal _config.yml
4. Verify basic build and serve functionality

Commands:

```bash
# Initialize Jekyll site
jekyll new mister-smith-site --blank
cd mister-smith-site

# Set up Gemfile
bundle init
bundle add jekyll
bundle add minimal-mistakes-jekyll
bundle add jekyll-feed
bundle add jekyll-sitemap
bundle add jekyll-seo-tag

# Install dependencies
bundle install
```

Validation:

- Site builds without errors: `bundle exec jekyll build`
- Site serves locally: `bundle exec jekyll serve`
- Basic Jekyll structure is in place

### Step 2: Collections Infrastructure Setup

Tasks:

1. Configure collections in _config.yml
2. Create collection directory structure
3. Set up basic collection configuration
4. Create placeholder index files

Collection Configuration (_config.yml):

```yaml
collections:
  core_architecture:
    output: true
    permalink: /:collection/:name/
  data_management:
    output: true
    permalink: /:collection/:name/
  operations:
    output: true
    permalink: /:collection/:name/
  research:
    output: true
    permalink: /:collection/:name/
  security:
    output: true
    permalink: /:collection/:name/
  transport:
    output: true
    permalink: /:collection/:name/
```

Directory Setup:

```bash
# Create collection directories
mkdir -p _core_architecture _data_management _operations _research _security _transport

# Create placeholder index files
touch _core_architecture/index.md
touch _data_management/index.md
touch _operations/index.md
touch _research/index.md
touch _security/index.md
touch _transport/index.md
```

Validation:

- Collections are recognized by Jekyll
- Collection URLs are accessible
- Directory structure is correct

### Step 3: Basic Theme Configuration

Tasks:

1. Configure Minimal Mistakes theme
2. Set up basic site configuration
3. Create minimal homepage
4. Configure basic navigation structure

Basic _config.yml Configuration:

```yaml
# Site settings
title: "Mister Smith AI Agent Framework"
description: "Multi-agent orchestration framework built with Rust"
url: "https://mattmagg.github.io"
baseurl: "/Mister-Smith"

# Theme
theme: minimal-mistakes-jekyll

# Build settings
markdown: kramdown
highlighter: rouge
permalink: /:categories/:title/

# Collections (from Step 2)
collections:
  core_architecture:
    output: true
    permalink: /:collection/:name/
  data_management:
    output: true
    permalink: /:collection/:name/
  operations:
    output: true
    permalink: /:collection/:name/
  research:
    output: true
    permalink: /:collection/:name/
  security:
    output: true
    permalink: /:collection/:name/
  transport:
    output: true
    permalink: /:collection/:name/

# Plugins
plugins:
  - jekyll-feed
  - jekyll-sitemap
  - jekyll-seo-tag
```

Basic Homepage (index.md):

```markdown
---
layout: home
title: "Mister Smith AI Agent Framework"
---

# Mister Smith AI Agent Framework

Multi-agent orchestration framework built with Rust, featuring NATS messaging, Claude integration, and supervision tree architecture for distributed AI agent coordination.

## Documentation Sections

- [Core Architecture](/core-architecture/)
- [Data Management](/data-management/)
- [Operations](/operations/)
- [Research](/research/)
- [Security](/security/)
- [Transport](/transport/)
```

Validation:

- Theme renders correctly
- Homepage displays properly
- Basic navigation works
- Site configuration is valid

### Step 4: GitHub Pages Deployment Setup

Tasks:

1. Configure repository for GitHub Pages
2. Set up basic deployment pipeline
3. Test foundation deployment

GitHub Pages Configuration:

1. Go to repository Settings > Pages
2. Select source: "Deploy from a branch"
3. Choose branch: main
4. Choose folder: / (root)
5. Save configuration

Basic .gitignore:

```gitignore
_site/
.sass-cache/
.jekyll-cache/
.jekyll-metadata
vendor/
```

Validation:

- Site deploys successfully to GitHub Pages
- Basic homepage loads correctly
- Collections are accessible
- No build errors in GitHub Actions

### Step 5: Foundation Testing

Tasks:

1. Verify all foundation components work
2. Test basic functionality
3. Confirm readiness for content and design work

Foundation Testing Checklist:

```bash
# Local testing
bundle exec jekyll build --strict_front_matter
bundle exec jekyll serve

# Check basic functionality
# - Homepage loads
# - Collections are accessible
# - Theme renders correctly
# - No build errors
```

Validation:

- Jekyll builds without errors
- All collection URLs are accessible
- Theme is properly configured
- Site is ready for content migration and design work

## Foundation Completion Criteria

### Technical Infrastructure

Jekyll Foundation:

- Jekyll site initializes and builds successfully
- Minimal Mistakes theme is properly configured
- Collections are set up and accessible
- Basic _config.yml is configured with essential settings
- GitHub Pages deployment pipeline is working

Directory Structure:

- All collection directories are created
- Basic index files exist for each collection
- Proper Jekyll directory structure is in place
- .gitignore is configured for Jekyll

### Deployment Pipeline

GitHub Pages:

- Repository is configured for GitHub Pages
- Site deploys automatically on push to main
- Basic homepage is accessible
- Collection URLs are working
- No build errors in deployment

### Readiness for Next Phases

Foundation provides:

- Stable Jekyll infrastructure for content migration
- Collection framework for organizing documentation
- Basic theme foundation for design customization
- Working deployment pipeline for continuous updates
- Testing framework for validation

## Next Phase Preparation

### Content Migration Phase (Future)

Foundation enables:

- Systematic migration of existing documentation
- Front matter standardization across documents
- Internal link preservation and updating
- Content organization within collections

### Design Implementation Phase (Future)

Foundation provides:

- Theme customization framework
- Navigation structure foundation
- Responsive design base
- Search functionality preparation

### Enhancement Phase (Future)

Foundation supports:

- Advanced feature implementation
- Performance optimization
- SEO enhancement
- Analytics integration

## Foundation Testing

### Local Testing

Basic Functionality:

```bash
# Build and serve locally
bundle exec jekyll build --strict_front_matter
bundle exec jekyll serve

# Verify basic functionality:
# - Homepage loads at http://localhost:4000
# - Collections are accessible
# - Theme renders correctly
# - No build errors or warnings
```

### Deployment Testing

GitHub Pages:

- Site deploys without errors
- Homepage is accessible at GitHub Pages URL
- Collection URLs work correctly
- No 404 errors for basic navigation

### Foundation Validation

Infrastructure Checklist:

- [ ] Jekyll site builds successfully
- [ ] Minimal Mistakes theme is active
- [ ] All 6 collections are configured
- [ ] Collection directories exist with placeholder files
- [ ] Basic _config.yml is properly configured
- [ ] GitHub Pages deployment works
- [ ] Homepage displays correctly
- [ ] Collection URLs are accessible

## Implementation Notes

### Prerequisites

- Jekyll and Ruby installed locally
- Git repository access
- Basic Jekyll knowledge
- GitHub Pages access

### Foundation Scope

This foundation plan establishes:

- Basic Jekyll infrastructure
- Collection framework
- Theme foundation
- Deployment pipeline
- Testing framework

### Out of Scope (Future Phases)

- Content migration from existing docs
- Advanced navigation implementation
- Custom design and styling
- Search functionality
- Performance optimization
- SEO implementation

### Success Criteria

Foundation is complete when:

- Jekyll site builds and deploys successfully
- All collections are accessible
- Basic theme is functional
- GitHub Pages deployment works
- Infrastructure is ready for content and design work
