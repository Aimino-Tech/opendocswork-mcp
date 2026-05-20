let activeFilter = 'all';

function filterCards() {
  const search = document.getElementById('search').value.toLowerCase();
  const cards = document.querySelectorAll('.card');

  cards.forEach(card => {
    const name = card.getAttribute('data-name') || '';
    const categories = card.getAttribute('data-category') || '';
    const title = card.querySelector('h3')?.textContent?.toLowerCase() || '';
    const desc = card.querySelector('p')?.textContent?.toLowerCase() || '';

    const matchesSearch = name.includes(search) || title.includes(search) || desc.includes(search);
    const matchesFilter = activeFilter === 'all' || categories.split(' ').includes(activeFilter);

    card.classList.toggle('hidden', !(matchesSearch && matchesFilter));
  });
}

function setFilter(filter) {
  activeFilter = filter;
  document.querySelectorAll('.filter-btn').forEach(btn => {
    btn.classList.toggle('active', btn.dataset.filter === filter);
  });
  filterCards();
}

// Copy JSON-RPC button handler
document.addEventListener('click', function(e) {
  if (e.target.classList.contains('copy-btn')) {
    const code = e.target.nextElementSibling?.querySelector('code')?.textContent || '';
    navigator.clipboard.writeText(code).then(() => {
      e.target.textContent = 'Copied!';
      setTimeout(() => { e.target.textContent = 'Copy'; }, 2000);
    });
  }
});

// Highlight.js initialization
document.addEventListener('DOMContentLoaded', () => {
  if (typeof hljs !== 'undefined') {
    hljs.highlightAll();
  }
});
