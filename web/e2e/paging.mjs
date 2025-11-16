import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'

const baseUrl = process.env.BASE_URL ?? 'http://127.0.0.1:58081'
const apiUrl = process.env.API_URL ?? 'http://127.0.0.1:58080'
const chromedriverPath = process.env.CHROMEDRIVER_PATH

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

async function buildDriver() {
  const options = new chrome.Options()
  options.addArguments('--headless=new', '--no-sandbox', '--disable-gpu')
  const builder = new Builder().forBrowser('chrome').setChromeOptions(options)
  if (chromedriverPath) {
    const service = new chrome.ServiceBuilder(chromedriverPath)
    builder.setChromeService(service)
  }
  return builder.build()
}

function button(text) {
  return By.xpath(`//button[normalize-space(.)='${text}']`)
}

async function mainText(driver) {
  return (await driver.findElement(By.css('main')).getText()).replace(/\s+/g, ' ')
}

async function run() {
  const driver = await buildDriver()
  const suffix = Date.now().toString(36)
  const username = `e2e_page_${suffix}`
  const marker = `paged${suffix}`
  try {
    await driver.get(`${baseUrl}/register`)
    await driver.wait(until.elementLocated(By.name('username')), 5000)
    await driver.findElement(By.name('username')).sendKeys(username)
    await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
    await driver.findElement(By.name('password')).sendKeys('correcthorse')
    await driver.findElement(By.css('button[type="submit"]')).click()
    await driver.wait(until.urlIs(`${baseUrl}/`), 5000)

    const cookie = await driver.manage().getCookie('session')
    const made = await driver.executeAsyncScript(
      `const cb = arguments[arguments.length - 1];
       const api = arguments[0], marker = arguments[1];
       (async () => {
         const ids = [];
         for (let n = 1; n <= 27; n++) {
           const r = await fetch(api + '/api/sections/general/topics', {
             method: 'POST', credentials: 'include',
             headers: { 'Content-Type': 'application/json' },
             body: JSON.stringify({ title: marker + ' topic ' + n, body: 'Body ' + n, tags: [marker] })
           });
           const t = await r.json();
           ids.push(t.id);
         }
         cb(ids);
       })();`,
      apiUrl,
      marker,
    )
    assert(made.length === 27 && cookie, 'enough topics should be created to force paging')
    const topicId = made[0]

    await driver.get(`${baseUrl}/s/general`)
    await driver.wait(until.elementLocated(button('Next')), 8000)
    let text = await mainText(driver)
    assert(/Page 1 of \d+/.test(text), `the section page should show its position, saw: ${text}`)

    const firstPage = text
    await driver.findElement(button('Next')).click()
    await driver.wait(async () => (await mainText(driver)).includes('Page 2 of'), 8000)
    const secondPage = await mainText(driver)
    assert(firstPage !== secondPage, 'the second page should show different topics')

    await driver.findElement(button('Previous')).click()
    await driver.wait(async () => (await mainText(driver)).includes('Page 1 of'), 8000)

    const prev = await driver.findElement(button('Previous'))
    assert(
      (await prev.getAttribute('disabled')) !== null,
      'Previous should be disabled on the first page',
    )

    await driver.get(`${baseUrl}/tag/${marker}`)
    await driver.wait(until.elementLocated(button('Next')), 8000)
    text = await mainText(driver)
    assert(/Page 1 of \d+/.test(text), `the tag page should paginate too, saw: ${text}`)

    const replies = await driver.executeAsyncScript(
      `const cb = arguments[arguments.length - 1];
       const api = arguments[0], topic = arguments[1];
       (async () => {
         for (let n = 1; n <= 27; n++) {
           const r = await fetch(api + '/api/topics/' + topic + '/comments', {
             method: 'POST', credentials: 'include',
             headers: { 'Content-Type': 'application/json' },
             body: JSON.stringify({ body: 'Root ' + String(n).padStart(2, '0') })
           });
           const c = await r.json();
           await fetch(api + '/api/topics/' + topic + '/comments', {
             method: 'POST', credentials: 'include',
             headers: { 'Content-Type': 'application/json' },
             body: JSON.stringify({ body: 'Reply to ' + String(n).padStart(2, '0'), parent_id: c.id })
           });
         }
         cb(true);
       })();`,
      apiUrl,
      topicId,
    )
    assert(replies === true, 'enough threaded comments should be created to force paging')

    await driver.get(`${baseUrl}/t/${topicId}`)
    await driver.wait(until.elementLocated(By.name('comment-body')), 8000)
    await driver.sleep(800)
    text = await mainText(driver)
    assert(text.includes('Root 01'), `the first comment page should show its roots, saw: ${text}`)
    assert(
      text.includes('Reply to 01'),
      'a reply must stay on the same page as its parent, never orphaned',
    )
    assert(!text.includes('Root 27'), 'a later root should not be on the first comment page')

    const commentNext = await driver.findElements(button('Next'))
    assert(commentNext.length > 0, 'the comment list should offer a next page')
    await commentNext[commentNext.length - 1].click()
    await driver.wait(async () => (await mainText(driver)).includes('Root 27'), 8000)
    text = await mainText(driver)
    assert(text.includes('Reply to 27'), 'the later page keeps its replies with their roots')
    assert(!text.includes('Root 01'), 'the first roots should be gone from the second page')

    const rejected = await driver.executeAsyncScript(
      `const cb = arguments[arguments.length - 1];
       fetch(arguments[0] + '/api/sections/general/topics?page=0')
         .then(r => cb(r.status)).catch(() => cb(0));`,
      apiUrl,
    )
    assert(rejected === 422, `an invalid page should be refused, got ${rejected}`)

    console.log('e2e: paging flow passed')
  } finally {
    await driver.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
