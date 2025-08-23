# Startup Map

README written by AI.

An interactive visualization of Y Combinator startups plotted on a 2D map based on their descriptions and similarities.

🌐 **Live Demo:** [https://patrik-cihal.github.io/startup-map](https://patrik-cihal.github.io/startup-map)

## Overview

This project creates an interactive map where Y Combinator startups are positioned based on semantic similarity of their descriptions. Companies with similar business models, target markets, or technologies appear closer together on the map.

## Features

- **Interactive Map**: Pan, zoom, and explore thousands of Y Combinator startups
- **Semantic Positioning**: Companies are positioned using AI embeddings and dimensionality reduction
- **Company Details**: Click on any startup to view details including name, tagline, team size, and logo
- **Responsive Visualization**: Built with Rust and WebAssembly for smooth performance
- **Dynamic Filtering**: Companies are filtered by team size based on zoom level for better readability

## Architecture

The project consists of three main components:

### 1. Data Scraping (`scraping/`)
- **Language**: Python
- **Purpose**: Scrapes Y Combinator company data including names, taglines, descriptions, and logos
- **Files**:
  - `scrape_links.py`: Extracts company links from YC directory
  - `scrape_details.py`: Fetches detailed company information
  - `requirements.txt`: Python dependencies

### 2. Embedding & Processing (`embedding/`)
- **Language**: Rust
- **Purpose**: Processes company descriptions into 2D coordinates for visualization
- **Key Features**:
  - Uses OpenAI API to normalize company taglines
  - Generates text embeddings using FastEmbed
  - Reduces dimensions using PaCMAP algorithm
  - Outputs CSV file with company positions

### 3. Visualization (`visualization/`)
- **Language**: Rust (Dioxus framework)
- **Purpose**: Interactive web application for exploring the startup map
- **Key Features**:
  - WebAssembly-powered for smooth performance
  - Pan and zoom controls
  - Dynamic company filtering based on zoom level
  - Company detail popups

## Getting Started

### Prerequisites

- Rust (latest stable version)
- Python 3.8+
- OpenAI API key (for embedding generation)

### Setup

1. **Clone the repository**:
   ```bash
   git clone https://github.com/patrik-cihal/startup-map.git
   cd startup-map
   ```

2. **Set up Python environment** (for scraping):
   ```bash
   cd scraping
   pip install -r requirements.txt
   ```

3. **Set up environment variables** (for embedding):
   ```bash
   # Create .env file in the embedding directory
   echo "OPENAI_API_KEY=your_api_key_here" > embedding/.env
   ```

### Running the Project

#### 1. Scrape Company Data (Optional)
```bash
cd scraping
python scrape_links.py      # Get company links
python scrape_details.py    # Get detailed company info
```

#### 2. Generate Embeddings and Positions
```bash
cd embedding
cargo run --release
```

#### 3. Run the Visualization
```bash
cd visualization
dx serve --platform web
```

The visualization will be available at `http://localhost:8080`.

### Building for Production

To build the WebAssembly version for deployment:

```bash
cd visualization
dx build --platform web --release
```

## Technology Stack

- **Frontend**: Dioxus (Rust web framework)
- **WebAssembly**: For high-performance web rendering
- **Embeddings**: FastEmbed for text embeddings
- **Dimensionality Reduction**: PaCMAP algorithm
- **Data Processing**: Rust with CSV handling
- **Web Scraping**: Python with Selenium and BeautifulSoup
- **AI Integration**: OpenAI API for tagline normalization

## Data Pipeline

1. **Scraping**: Extract company data from Y Combinator directory
2. **Normalization**: Use OpenAI to standardize company taglines
3. **Embedding**: Generate semantic embeddings for company descriptions
4. **Dimensionality Reduction**: Use PaCMAP to create 2D coordinates
5. **Visualization**: Render interactive map with WebAssembly

## Configuration

### Embedding Configuration
- Model: Default FastEmbed model for text embeddings
- Dimensionality reduction: PaCMAP with optimized parameters
- Output format: CSV with company positions and metadata

### Visualization Configuration
- Zoom-based filtering for better performance
- Company size thresholds for different zoom levels
- Smooth pan/zoom animations

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is open source and available under the [MIT License](LICENSE).

## Acknowledgments

- Y Combinator for providing startup data
- OpenAI for embedding generation capabilities
- The Rust community for excellent WebAssembly tooling
- PaCMAP authors for the dimensionality reduction algorithm
